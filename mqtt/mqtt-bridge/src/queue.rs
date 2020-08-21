use indexmap::map::Iter;
use indexmap::IndexMap;
use std::{collections::BTreeMap, iter::Iterator, time::Duration};

use anyhow::Error;
use mqtt3::proto::Publication;

use thiserror::Error;

// TODO: change to string
trait Queue {
    type Loader: MessageLoader;

    // TODO: name?
    fn new() -> Self;

    fn insert(self, priority: u32, ttl: Duration, message: Publication) -> Result<u32, Error>;

    fn remove(self, key: u32) -> Result<bool, Error>;

    fn batch_iter(self, count: usize) -> MovingWindowIter<Self::Loader>;
}

trait MessageLoader {
    type Iter: Iterator<Item = (u32, Publication)>;

    fn range(&self, count: u32) -> Self::Iter;
}

struct MovingWindowIter<L>
where
    L: MessageLoader,
{
    count: usize,

    messages: L,
}

// struct MovingWindowIter {
//     count: usize,

//     messages: Iter<Item = (u32, Publication)>,
// }

struct SimpleMessageLoader {
    messages: Iter<u32, Publication>,
}

// type Iter = Iter<u32, Publication>
impl MessageLoader for SimpleMessageLoader {
    type Iter = Iter<u32, Publication>;

    fn range(&self, count: u32) -> Iter<u32, Publication> {
        // TODO: return specific amount of messages?
        self.messages
    }
}

struct SimpleQueue {
    messages: IndexMap<u32, Publication>,
    offset: u32,
}

impl Queue for SimpleQueue {
    // type Loader =
    fn new() -> SimpleQueue {
        let messages: IndexMap<u32, Publication> = IndexMap::new();
        let offset = 0;
        SimpleQueue { messages, offset }
    }

    fn insert(self, priority: u32, ttl: Duration, message: Publication) -> Result<u32, Error> {
        self.messages.insert(self.offset, message);

        let output = self.offset;
        self.offset += 1;

        Ok(output)
    }

    fn remove(self, key: u32) -> Result<bool, Error> {
        self.messages
            .remove(&key)
            .ok_or(QueueError::RemovalFailure())?;

        Ok(true)
    }

    fn batch_iter(self, count: usize) -> MovingWindowIter<Self::Loader> {
        MovingWindowIter {
            messages: self.messages.iter(),
            count,
        }
    }
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
