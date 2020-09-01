use std::cell::RefCell;
use std::collections::btree_map::Range;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::sync::Arc;
use std::task::Waker;
use std::{iter::Iterator, time::Duration};

use anyhow::{Error, Result};
use async_trait::async_trait;
use mqtt3::proto::Publication;
// TODO: do we need this tokio mutex
use tokio::sync::Mutex;

use crate::queue::{simple_message_loader::SimpleMessageLoader, Key, Queue, QueueError};

// TODO: wrap map operations in methods?
pub struct WakingBTreeMap {
    pub map: BTreeMap<Key, Publication>,
    pub waker: Option<Waker>,
}

impl WakingBTreeMap {
    pub fn new(map: BTreeMap<Key, Publication>) -> Self {
        WakingBTreeMap { map, waker: None }
    }
}

struct SimpleQueue {
    state: Arc<Mutex<WakingBTreeMap>>,
    offset: u32,
}

#[async_trait]
impl<'a> Queue<'a> for SimpleQueue {
    type Loader = SimpleMessageLoader;

    fn new() -> Self {
        let waking_map = WakingBTreeMap::new(BTreeMap::new());
        let state = Arc::new(Mutex::new(waking_map));
        let offset = 0;
        SimpleQueue { state, offset }
    }

    async fn insert(
        &mut self,
        priority: u32,
        ttl: Duration,
        message: Publication,
    ) -> Result<Key, QueueError> {
        let key = Key {
            offset: self.offset,
            priority,
            ttl,
        };

        let mut state_lock = self.state.lock().await;
        state_lock.map.insert(key.clone(), message);
        if let Some(waker) = state_lock.waker.clone() {
            waker.wake();
        }

        self.offset += 1;
        Ok(key)
    }

    async fn remove(&mut self, key: Key) -> Result<bool, QueueError> {
        let mut state_lock = self.state.lock().await;
        state_lock.map.remove(&key).ok_or(QueueError::Removal())?;

        Ok(true)
    }

    async fn get_loader(&'a mut self, batch_size: usize) -> SimpleMessageLoader {
        SimpleMessageLoader::new(Arc::clone(&self.state), batch_size).await
    }
}

// TODO: test errors
// TODO: test remove maintains ordering
// TODO: add tests for different loaders sizes
// #[cfg(test)]
// mod tests {
//     use std::time::Duration;

//     use bytes::Bytes;
//     use mqtt3::proto::{Publication, QoS};

//     use crate::queue::{simple_queue::SimpleQueue, Queue};

//     #[test]
//     fn insert() {
//         let mut queue = SimpleQueue::new();
//         let publication = Publication {
//             topic_name: "test".to_string(),
//             qos: QoS::ExactlyOnce,
//             retain: true,
//             payload: Bytes::new(),
//         };

//         queue
//             .insert(0, Duration::from_secs(30), publication.clone())
//             .expect("failed to insert message into queue");

//         let message_loader = queue.get_loader(3);
//         let extracted: &(String, Publication) = message_loader.next().unwrap();

//         assert_ne!("", (*extracted).0);
//         assert_eq!((*extracted).1, publication);
//     }

//     #[test]
//     fn iter_multiple() {
//         let mut queue = SimpleQueue::new();
//         let publication = Publication {
//             topic_name: "test".to_string(),
//             qos: QoS::ExactlyOnce,
//             retain: true,
//             payload: Bytes::new(),
//         };

//         queue
//             .insert(0, Duration::from_secs(30), publication.clone())
//             .expect("failed to insert message into queue");

//         queue
//             .insert(0, Duration::from_secs(30), publication.clone())
//             .expect("failed to insert message into queue");

//         let message_loader = queue.iter(2);
//         let mut iter = message_loader.range(2).unwrap();
//         let extracted_first: &(String, Publication) = iter.next().unwrap();
//         let extracted_second: &(String, Publication) = iter.next().unwrap();

//         // check that keys are different but pubs are same
//         assert_ne!((*extracted_first).0, (*extracted_second).0);
//         assert_eq!(publication, (*extracted_first).1);
//         assert_eq!(publication, (*extracted_second).1);
//     }

//     #[test]
//     fn insert_maintains_order() {
//         let mut queue = SimpleQueue::new();

//         // Vec<Publication>::new()
//         let mut pubs = vec![];
//         let num_messages = 50;
//         for count in 0..num_messages {
//             let publication = Publication {
//                 topic_name: count.to_string(),
//                 qos: QoS::ExactlyOnce,
//                 retain: true,
//                 payload: Bytes::new(),
//             };

//             pubs.push(publication.clone());

//             queue
//                 .insert(0, Duration::from_secs(30), publication.clone())
//                 .expect("failed to insert message into queue");
//         }

//         let message_loader = queue.iter(num_messages);
//         let mut iter = message_loader.range(num_messages).unwrap();

//         for count in 0..num_messages {
//             let pub_from_arr = (*pubs.get(count).unwrap()).clone();
//             let pub_from_queue = iter.next().unwrap();

//             assert_eq!(pub_from_arr, pub_from_queue.1);
//         }
//     }

//     #[test]
//     fn remove() {
//         let mut queue = SimpleQueue::new();
//         let publication = Publication {
//             topic_name: "test".to_string(),
//             qos: QoS::ExactlyOnce,
//             retain: true,
//             payload: Bytes::new(),
//         };

//         queue
//             .insert(0, Duration::from_secs(30), publication.clone())
//             .expect("failed to insert message into queue");

//         let message_loader = queue.iter(1);
//         let extracted: &(String, Publication) = message_loader.range(1).unwrap().next().unwrap();

//         // TODO: if queue.remove() takes a &str then we won't have to clone
//         let key = (*extracted).0.clone();
//         queue.remove(key).expect("failed to remove from queue");

//         assert_eq!(queue.iter(1).range(1).unwrap().next(), None);
//     }
// }
