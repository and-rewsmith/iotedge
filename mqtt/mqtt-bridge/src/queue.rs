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

    fn insert(
        &mut self,
        priority: u32,
        ttl: Duration,
        message: Publication,
    ) -> Result<String, Error>;

    fn remove(&mut self, key: String) -> Result<bool, Error>;

    fn iter(&mut self) -> Self::Loader;

    // fn batch_iter(self, count: usize) -> MovingWindowIter<Self::Loader>;
}

// trait MessageLoader {
//     type Iter: Iterator<Item = (u32, Publication)>;

//     fn range(&self, count: u32) -> Self::Iter;
// }

trait MessageLoader {
    fn range(&self, count: u32) -> Iter<String, Publication>;
}

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
    // messages: Iter<'static, String, Publication>,
    messages: IndexMap<String, Publication>,
}

// type Iter = Iter<u32, Publication>

// impl MessageLoader for SimpleMessageLoader {
//     type Iter = Iter<String, Publication>;

//     fn range(&self, count: u32) -> Iter<u32, Publication> {
//         // TODO: return specific amount of messages?
//         self.messages.iter();
//     }
// }
impl MessageLoader for SimpleMessageLoader {
    fn range(&self, count: u32) -> Iter<String, Publication> {
        // TODO: return specific amount of messages?
        // TODO: do not clone
        self.messages.iter()
    }
}

struct SimpleQueue {
    messages: IndexMap<String, Publication>,
    offset: u32,
}

impl Queue for SimpleQueue {
    type Loader = SimpleMessageLoader;

    fn new() -> SimpleQueue {
        let messages: IndexMap<String, Publication> = IndexMap::new();
        let offset = 0;
        SimpleQueue { messages, offset }
    }

    fn insert(
        &mut self,
        priority: u32,
        ttl: Duration,
        message: Publication,
    ) -> Result<String, Error> {
        self.messages.insert(self.offset.to_string(), message);

        self.offset += 1;

        Ok(self.offset.to_string())
    }

    fn remove(&mut self, key: String) -> Result<bool, Error> {
        self.messages
            .remove(&key)
            .ok_or(QueueError::RemovalFailure())?;

        Ok(true)
    }

    // TODO: do not clone
    fn iter(&mut self) -> SimpleMessageLoader {
        // let messages_copy = self.messages.clone();
        // let iter = messages_copy.iter();
        SimpleMessageLoader {
            messages: self.messages.clone(),
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
