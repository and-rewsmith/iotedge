use std::cell::RefCell;
use std::cmp::min;
use std::collections::btree_map::Range;
use std::collections::BTreeMap;
use std::iter::Take;
use std::pin::Pin;
use std::rc::Rc;
use std::slice::Iter;
use std::task::Context;
use std::{task::Poll, vec::IntoIter};

use anyhow::Result;
use futures_util::stream::Stream;
use mqtt3::proto::Publication;

use crate::queue::{Key, QueueError};

pub struct SimpleMessageLoader<'a> {
    state: &'a RefCell<BTreeMap<Rc<Key>, Rc<Publication>>>,
    batch: IntoIter<(Rc<Key>, Rc<Publication>)>,
    batch_size: usize,
}

impl<'a> SimpleMessageLoader<'a> {
    pub fn new(state: &'a RefCell<BTreeMap<Rc<Key>, Rc<Publication>>>, batch_size: usize) -> Self {
        let batch: Vec<_> = state
            .borrow()
            .iter()
            .take(batch_size)
            .map(|element| (element.0.clone(), element.1.clone()))
            .collect();
        let batch = batch.into_iter();

        SimpleMessageLoader {
            state,
            batch,
            batch_size,
        }
    }
}

impl<'a> Stream for SimpleMessageLoader<'a> {
    type Item = (Rc<Key>, Rc<Publication>);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if let Some(item) = self.batch.next() {
            return Poll::Ready(Some((item.0.clone(), item.1.clone())));
        }

        let new_elements: Vec<_> = self
            .state
            .borrow()
            .iter()
            .take(self.batch_size)
            .map(|element| (element.0.clone(), element.1.clone()))
            .collect();
        self.batch = new_elements.into_iter();

        self.batch.next().map_or_else(
            || Poll::Pending,
            |item| Poll::Ready(Some((item.0.clone(), item.1.clone()))),
        )
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
// TODO: tests with different batch sizes
// TODO: tests with no removal
// TODO: test with no elements
#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::iter::Iterator;
    use std::rc::Rc;
    use std::time::Duration;

    use bytes::Bytes;
    use mqtt3::proto::{Publication, QoS};

    use futures_util::stream::Stream;
    use futures_util::stream::StreamExt;

    use crate::queue::simple_message_loader::SimpleMessageLoader;
    use crate::queue::{Key, QueueError};

    #[tokio::test]
    async fn happy_path_retrieval() {
        let key1 = Key {
            priority: 0,
            offset: 0,
            ttl: Duration::from_secs(5),
        };
        let publication1 = Publication {
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
        let publication2 = Publication {
            topic_name: "test2".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        let map = BTreeMap::new();
        let map = RefCell::new(map);

        map.borrow_mut()
            .insert(Rc::new(key1.clone()), Rc::new(publication1.clone()));
        map.borrow_mut()
            .insert(Rc::new(key2.clone()), Rc::new(publication2.clone()));

        let batch_size = 5;
        let mut loader = SimpleMessageLoader::new(&map, batch_size);

        // while let Some(extracted) = loader.next().await {}
        let extracted1 = loader.next().await.unwrap();
        let extracted2 = loader.next().await.unwrap();

        // make sure same publications come out in correct order
        assert_eq!(*extracted1.0, key1);
        assert_eq!(*extracted2.0, key2);
        assert_eq!(*extracted1.1, publication1);
        assert_eq!(*extracted2.1, publication2);

        // TODO: might want to delete this
        // if caller successfully handles messages, expectation is they will delete from btree
        map.borrow_mut().remove(&key1.clone());
        map.borrow_mut().remove(&key2.clone());
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
