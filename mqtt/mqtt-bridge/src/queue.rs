use indexmap::map::Iter;
use indexmap::IndexMap;
// use std::collections::hash_map::Iter;
use std::{collections::BTreeMap, collections::HashMap, iter::Iterator, time::Duration};

use anyhow::Error;
use mqtt3::proto::Publication;

use thiserror::Error;

// TODO: change to string
trait Queue {
    type Loader: MessageLoader;

    // TODO: name?
    fn new() -> Self;

    fn insert(self, priority: u32, ttl: Duration, message: Publication) -> Result<String, Error>;

    fn remove(self, key: String) -> Result<bool, Error>;

    fn iter(self, count: usize) -> Self::Loader;

    // fn batch_iter(self, count: usize) -> MovingWindowIter<Self::Loader>;
}

trait MessageLoader {
    type Iter: Iterator<Item = (u32, Publication)>;

    fn range(&self, count: u32) -> Self::Iter;
}

// trait MessageLoader {
//     fn range(&self, count: u32) -> Iter<String, Publication>;
// }

// From spec:
// struct MovingWindowIter<L>
// where
//     L: MessageLoader,
// {
//     count: usize,

//     messages: L,
// }

// Manual test:
// struct MovingWindowIter {
//     count: usize,

//     messages: Iter<Item = (u32, Publication)>,
// }

struct SimpleMessageLoader {
    messages: IndexMap<String, Publication>,
}

// type Iter = Iter<u32, Publication>

impl MessageLoader for SimpleMessageLoader {
    type Iter = Iter<String, Publication>;

    fn range(&self, count: u32) -> Iter<u32, Publication> {
        // TODO: return specific amount of messages?
        self.messages.iter();
    }
}

struct SimpleQueue {
    messages: IndexMap<String, Publication>,
    offset: u32,
}

impl Queue for SimpleQueue {
    type Loader = SimpleMessageLoader;

    fn new() -> SimpleQueue {
        let messages: IndexMap<u32, Publication> = IndexMap::new();
        let offset = 0;
        SimpleQueue { messages, offset }
    }

    fn insert(self, priority: u32, ttl: Duration, message: Publication) -> Result<String, Error> {
        let output_key = String::to_string(self.offset);
        self.messages.insert(output_key, message);

        self.offset += 1;

        Ok(output_key)
    }

    fn remove(self, key: String) -> Result<bool, Error> {
        self.messages
            .remove(&key)
            .ok_or(QueueError::RemovalFailure())?;

        Ok(true)
    }

    fn iter(self, count: usize) -> SimpleMessageLoader {
        SimpleMessageLoader {
            messages: self.messages.iter(),
        }
    }

    // fn batch_iter(self, count: usize) -> MovingWindowIter<Self::Loader> {
    //     MovingWindowIter {
    //         messages: self.messages.iter(),
    //         count,
    //     }
    // }
}

#[derive(Debug, Error)]
enum QueueError {
    #[error("Failed to remove messages from queue")]
    RemovalFailure(),
}

// struct MessageLoader {
// }

// impl MessageLoader for MessageLoader {
//     fn range(&self, count: u32) {

//     }
// }
