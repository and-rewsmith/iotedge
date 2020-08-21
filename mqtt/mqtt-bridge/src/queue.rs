use indexmap::map::Iter;
use indexmap::IndexMap;
use std::{iter::Iterator, time::Duration};

use anyhow::Error;
use mqtt3::proto::Publication;

use thiserror::Error;

trait Queue {
    type Loader: MessageLoader;

    // TODO: add name as per spec?
    fn new() -> Self;

    fn insert(
        &mut self,
        priority: u32,
        ttl: Duration,
        message: Publication,
    ) -> Result<String, Error>;

    fn remove(&mut self, key: String) -> Result<bool, Error>;

    fn iter(&mut self) -> Self::Loader;

    // TODO: what is the point of this func defined in the spec?
    // fn batch_iter(self, count: usize) -> MovingWindowIter<Self::Loader>;
}

// TODO: should the iterator here be going over refs instead of values?
trait MessageLoader {
    // type Iter: Iterator<Item = (String, Publication)>;
    type Iter: Iterator<Item = (&'static String, &'static Publication)>;

    fn range(&self, count: u32) -> Self::Iter;
}

// TODO: What is the point of this sliding window?
// ###############################
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
// ###############################

// TODO: should we be using index map's iter? It returns items with refs which fails compilation
// TODO: do not store the messages themselves
struct SimpleMessageLoader {
    messages: IndexMap<String, Publication>,
}

impl MessageLoader for SimpleMessageLoader {
    // TODO: should lifetime be static
    type Iter = Iter<'static, String, Publication>;

    fn range(&self, count: u32) -> Iter<'static, String, Publication> {
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
