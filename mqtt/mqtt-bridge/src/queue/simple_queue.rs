use std::cmp::min;
use std::{iter::Iterator, time::Duration};

use anyhow::Error;
use anyhow::Result;
use core::slice::Iter;
use indexmap::IndexMap;
use mqtt3::proto::Publication;

use crate::queue::MessageLoader;
use crate::queue::Queue;
use crate::queue::QueueError;

struct SimpleMessageLoader {
    messages: Vec<(String, Publication)>,
}

impl<'a> MessageLoader<'a> for SimpleMessageLoader {
    type Iter = Iter<'a, (String, Publication)>;

    fn range(&'a self, count: usize) -> Result<Iter<'a, (String, Publication)>> {
        let output_cardinality = min(self.messages.len(), count);

        Ok(self
            .messages
            .get(0..output_cardinality)
            .ok_or(QueueError::LoadMessage())?
            .into_iter())
    }
}

struct SimpleQueue {
    state: IndexMap<String, Publication>,
    offset: u32,
}

impl Queue for SimpleQueue {
    type Loader = SimpleMessageLoader;

    fn new() -> Self {
        let messages: IndexMap<String, Publication> = IndexMap::new();
        let offset = 0;
        SimpleQueue {
            state: messages,
            offset,
        }
    }

    fn insert(
        &mut self,
        priority: u32,
        ttl: Duration,
        message: Publication,
    ) -> Result<String, Error> {
        self.state.insert(self.offset.to_string(), message);

        self.offset += 1;

        Ok(self.offset.to_string())
    }

    fn remove(&mut self, key: String) -> Result<bool, Error> {
        self.state.remove(&key).ok_or(QueueError::Removal())?;

        Ok(true)
    }

    // TODO: do not clone
    fn iter(&mut self, count: usize) -> SimpleMessageLoader {
        let mut iter = self.state.iter();
        let mut output = vec![];

        let mut messages_found = 0;
        while let Some(pair) = iter.next() {
            if messages_found == count {
                break;
            }

            output.push((pair.0.clone(), pair.1.clone()));
            messages_found += 1;
        }

        SimpleMessageLoader { messages: output }
    }

    // TODO: implement this batch iter func as per spec
    // fn batch_iter(self, count: usize) -> MovingWindowIter<Self::Loader> {}
}

// TODO: test basic scenarios (insert, removal, iter)
// TODO: test ordered entries
// TODO: ttl
// TODO: test errors
#[cfg(test)]
mod tests {
    use std::time::Duration;

    use anyhow::Error;
    use bytes::Bytes;
    use mqtt3::proto::Publication;
    use mqtt3::proto::QoS;

    use crate::queue::simple_queue::SimpleQueue;
    use crate::queue::MessageLoader;
    use crate::queue::Queue;

    #[test]
    fn insert() {
        let mut queue = SimpleQueue::new();
        let publication = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        queue
            .insert(0, Duration::from_secs(30), publication.clone())
            .expect("failed to insert message into queue");

        let message_loader = queue.iter(1);
        let mut iter = message_loader.range(2).unwrap();
        let extracted: &(String, Publication) = iter.next().unwrap();

        assert_ne!("", (*extracted).0);
        assert_eq!((*extracted).1, publication);
    }

    #[test]
    fn iter_multiple() {
        let mut queue = SimpleQueue::new();
        let publication = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        queue
            .insert(0, Duration::from_secs(30), publication.clone())
            .expect("failed to insert message into queue");

        queue
            .insert(0, Duration::from_secs(30), publication.clone())
            .expect("failed to insert message into queue");

        let message_loader = queue.iter(2);
        let mut iter = message_loader.range(2).unwrap();
        let extracted_first: &(String, Publication) = iter.next().unwrap();
        let extracted_second: &(String, Publication) = iter.next().unwrap();

        // check that keys are different but pubs are same
        assert_ne!((*extracted_first).0, (*extracted_second).0);
        assert_eq!(publication, (*extracted_first).1);
        assert_eq!(publication, (*extracted_second).1);
    }

    #[test]
    fn remove() {
        let mut queue = SimpleQueue::new();
        let publication = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        queue
            .insert(0, Duration::from_secs(30), publication.clone())
            .expect("failed to insert message into queue");

        let message_loader = queue.iter(1);
        let extracted: &(String, Publication) = message_loader.range(1).unwrap().next().unwrap();

        // TODO: if queue.remove() takes a &str then we won't have to clone
        let key = (*extracted).0.clone();
        queue.remove(key).expect("failed to remove from queue");

        assert_eq!(queue.iter(1).range(1).unwrap().next(), None);
    }
}

// pub struct Publication {
//     pub topic_name: String,
//     pub qos: crate::proto::QoS,
//     pub retain: bool,
//     #[cfg_attr(feature = "serde1", serde(serialize_with = "serialize_bytes"))]
//     #[cfg_attr(feature = "serde1", serde(deserialize_with = "deserialize_bytes"))]
//     pub payload: bytes::Bytes,
// }
