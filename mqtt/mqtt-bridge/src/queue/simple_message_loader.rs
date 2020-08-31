use std::cell::RefCell;
use std::cmp::min;
use std::collections::btree_map::Range;
use std::collections::BTreeMap;
use std::iter::Take;
use std::pin::Pin;
use std::rc::Rc;
use std::slice::Iter;
use std::sync::Arc;
use std::task::Context;
use std::{task::Poll, vec::IntoIter};

use anyhow::Result;
use futures_util::stream::Stream;
use mqtt3::proto::Publication;
use tokio::sync::Mutex;

use crate::queue::{Key, QueueError};

// TODO: should this have some way of shutting down? Callers reading stream will hang?
pub struct SimpleMessageLoader {
    state: Arc<Mutex<BTreeMap<Key, Publication>>>,
    batch: IntoIter<(Key, Publication)>,
    batch_size: usize,
}

impl SimpleMessageLoader {
    pub async fn new(state: Arc<Mutex<BTreeMap<Key, Publication>>>, batch_size: usize) -> Self {
        let state_lock = state.lock().await;
        let batch: Vec<_> = state_lock
            .iter()
            .take(batch_size)
            .map(|element| (element.0.clone(), element.1.clone()))
            .collect();
        let batch = batch.into_iter();

        SimpleMessageLoader {
            state: Arc::clone(&state),
            batch,
            batch_size,
        }
    }
}

impl Stream for SimpleMessageLoader {
    type Item = (Key, Publication);

    // TODO: remove all println
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        println!("in poll next");

        if let Some(item) = self.batch.next() {
            println!("detected item");
            return Poll::Ready(Some((item.0.clone(), item.1.clone())));
        }

        let mut_self = self.get_mut();
        if let Ok(state) = mut_self.state.try_lock() {
            println!("refreshing batch");
            // TODO: extract to function
            let batch: Vec<_> = state
                .iter()
                .take(mut_self.batch_size)
                .map(|element| (element.0.clone(), element.1.clone()))
                .collect();
            mut_self.batch = batch.into_iter();

            // TODO: store waker in this struct, refactor queue to store loader, then call loaders wake method in insert (issues with ownership)
            // TODO: or... make wrapper type around btreemap that calls potential wakers on insert. here store waker in this struct. no issues with ownership.
            println!("returning from poll next");
            return mut_self.batch.next().map_or_else(
                || {
                    cx.waker().wake()
                    Poll::Pending
                },
                |item| Poll::Ready(Some((item.0.clone(), item.1.clone()))),
            );
        }

        println!("didn't acquire lock");
        Poll::Pending
    }
}

// impl<'a> MessageLoader<'a> for SimpleMessageLoader {
//     type Iter = Range<'a, String, Publication>;

//     fn range(&'a self, keys: Range<String, Publication>) -> Result<Range<'a, String, Publication>> {
//         // let output_cardinality = min(self.messages.len(), );

//         // Ok(self
//         //     .messages
//         //     .get(0..output_cardinality)
//         //     .ok_or(QueueError::LoadMessage())?
//         //     .into_iter())

//         // self.state.range(keys)

//         Ok(self.state.range(keys))
//     }
// }

// TODO: consolidate logic
/*
TESTS:

happy path

different btree map sizes
- populated
- unpopulated

different batch sizes
- 0
- 1
- 5
-1000

relative sizes
- batch size > btree map
- batch size < btree map

no elements in the loader

ordering is maintained across inserts

ordering is maintained across deletes

constant writes make sure stream is able to resolve

*/
#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::iter::Iterator;
    use std::rc::Rc;
    use std::sync::Arc;
    use std::time::Duration;

    // use async_std::task;
    use async_std::task;
    use bytes::Bytes;
    use futures_util::stream::Stream;
    use futures_util::stream::StreamExt;
    use mqtt3::proto::{Publication, QoS};
    use tokio;
    use tokio::sync::Mutex;
    use tokio::time;

    use crate::queue::simple_message_loader::SimpleMessageLoader;
    use crate::queue::{Key, QueueError};

    #[tokio::test]
    async fn retrieve_elements() {
        let key1 = Key {
            priority: 0,
            offset: 0,
            ttl: Duration::from_secs(5),
        };
        let pub1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };
        let key2 = Key {
            priority: 0,
            offset: 1,
            ttl: Duration::from_secs(5),
        };
        let pub2 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        let map = BTreeMap::new();
        let map = Arc::new(Mutex::new(map));

        map.lock().await.insert(key1.clone(), pub1.clone());
        map.lock().await.insert(key2.clone(), pub2.clone());

        let batch_size = 5;
        let mut loader = SimpleMessageLoader::new(Arc::clone(&map), batch_size).await;

        let extracted1 = loader.next().await.unwrap();
        let extracted2 = loader.next().await.unwrap();

        // make sure same publications come out in correct order
        assert_eq!(extracted1.0, key1);
        assert_eq!(extracted2.0, key2);
        assert_eq!(extracted1.1, pub1);
        assert_eq!(extracted2.1, pub2);

        // TODO: might want to delete this
        // if caller successfully handles messages, expectation is they will delete from btree
        // map.borrow_mut().remove(&key1.clone());
        // map.borrow_mut().remove(&key2.clone());
    }

    // TODO: remove all println
    #[tokio::test]
    async fn poll_stream_does_not_block_when_map_empty() {
        let key1 = Key {
            priority: 0,
            offset: 0,
            ttl: Duration::from_secs(5),
        };
        let pub1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        let map: BTreeMap<Key, Publication> = BTreeMap::new();
        let map = Arc::new(Mutex::new(map));

        let batch_size = 5;
        let mut loader = SimpleMessageLoader::new(Arc::clone(&map), batch_size).await;

        let key_copy = key1.clone();
        let pub_copy = pub1.clone();
        let poll_stream = async move {
            println!("beginning poll stream");
            let maybe_extracted = loader.next().await;
            if let Some(extracted) = maybe_extracted {
                println!("finished polling stream");
                assert_eq!((key_copy, pub_copy), extracted);
                println!("finished assert");
            } else {
                println!("got none");
            }
        };

        let poll_stream_handle = tokio::spawn(poll_stream);

        time::delay_for(Duration::from_secs(2)).await;

        let mut map_lock = map.lock().await;
        map_lock.insert(key1, pub1);
        println!("waiting for poll_stream");
        drop(map_lock);
        poll_stream_handle.await.unwrap();
    }

    // #[test]
    // fn retrieve_more_than_exists() {
    //     let key1 = "key1".to_string();
    //     let publication1 = Publication {
    //         topic_name: "test".to_string(),
    //         qos: QoS::ExactlyOnce,
    //         retain: true,
    //         payload: Bytes::new(),
    //     };

    //     let key2 = "key2".to_string();
    //     let publication2 = Publication {
    //         topic_name: "test2".to_string(),
    //         qos: QoS::ExactlyOnce,
    //         retain: true,
    //         payload: Bytes::new(),
    //     };

    //     let messages = vec![
    //         (key1.clone(), publication1.clone()),
    //         (key2.clone(), publication2.clone()),
    //     ];

    //     let loader = SimpleMessageLoader::new(messages);

    //     let mut iter = loader.range(5).unwrap();
    //     let extracted1 = iter.next().unwrap();
    //     let extracted2 = iter.next().unwrap();

    //     // make sure same publications come out in correct order
    //     assert_eq!((*extracted1).0, key1);
    //     assert_eq!((*extracted2).0, key2);
    //     assert_eq!((*extracted1).1, publication1);
    //     assert_eq!((*extracted2).1, publication2);

    //     // make sure no more elements
    //     assert_eq!(iter.next(), None);
    // }

    // #[test]
    // fn retrieve_when_empty() {
    //     let messages = vec![];
    //     let loader = SimpleMessageLoader::new(messages);

    //     let mut iter = loader.range(5).unwrap();

    //     // make sure no more elements
    //     assert_eq!(iter.next(), None);
    // }
}
