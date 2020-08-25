use std::cmp::min;
use std::collections::btree_map::Range;
use std::collections::BTreeMap;

use anyhow::Result;
use core::slice::Iter;
use mqtt3::proto::Publication;

use crate::queue::{MessageLoader, QueueError};

pub struct SimpleMessageLoader {
    state: BTreeMap<String, Publication>,
}

impl SimpleMessageLoader {
    pub fn new(state: BTreeMap<String, Publication>) -> Self {
        SimpleMessageLoader { state }
    }
}

impl<'a> MessageLoader<'a> for SimpleMessageLoader {
    type Iter = Range<'a, String, Publication>;

    fn range(&'a self, keys: Range<String, Publication>) -> Result<Range<'a, String, Publication>> {
        // let output_cardinality = min(self.messages.len(), );

        // Ok(self
        //     .messages
        //     .get(0..output_cardinality)
        //     .ok_or(QueueError::LoadMessage())?
        //     .into_iter())

        // self.state.range(keys)

        Ok(self.state.range(keys))
    }
}

// TODO: consolidate logic
#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use mqtt3::proto::{Publication, QoS};

    use crate::queue::{simple_message_loader::SimpleMessageLoader, MessageLoader};

    #[test]
    fn retrieve() {
        let key1 = "key1".to_string();
        let publication1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        let key2 = "key2".to_string();
        let publication2 = Publication {
            topic_name: "test2".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        let messages = vec![
            (key1.clone(), publication1.clone()),
            (key2.clone(), publication2.clone()),
        ];

        let loader = SimpleMessageLoader::new(messages);

        let mut iter = loader.range(2).unwrap();
        let extracted1 = iter.next().unwrap();
        let extracted2 = iter.next().unwrap();

        // make sure same publications come out in correct order
        assert_eq!((*extracted1).0, key1);
        assert_eq!((*extracted2).0, key2);
        assert_eq!((*extracted1).1, publication1);
        assert_eq!((*extracted2).1, publication2);

        // make sure no more elements
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn retrieve_more_than_exists() {
        let key1 = "key1".to_string();
        let publication1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        let key2 = "key2".to_string();
        let publication2 = Publication {
            topic_name: "test2".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        let messages = vec![
            (key1.clone(), publication1.clone()),
            (key2.clone(), publication2.clone()),
        ];

        let loader = SimpleMessageLoader::new(messages);

        let mut iter = loader.range(5).unwrap();
        let extracted1 = iter.next().unwrap();
        let extracted2 = iter.next().unwrap();

        // make sure same publications come out in correct order
        assert_eq!((*extracted1).0, key1);
        assert_eq!((*extracted2).0, key2);
        assert_eq!((*extracted1).1, publication1);
        assert_eq!((*extracted2).1, publication2);

        // make sure no more elements
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn retrieve_when_empty() {
        let messages = vec![];
        let loader = SimpleMessageLoader::new(messages);

        let mut iter = loader.range(5).unwrap();

        // make sure no more elements
        assert_eq!(iter.next(), None);
    }
}
