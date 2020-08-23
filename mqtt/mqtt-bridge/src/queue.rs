// TODO: split interface and impl into separate files

// use indexmap::map::Iter;
use core::slice::Iter;
use indexmap::IndexMap;
use std::{iter::Iterator, time::Duration};

use anyhow::Error;
use mqtt3::proto::Publication;

use thiserror::Error;

// TODO: add lifetimes
trait Queue {
    type Loader: MessageLoader<'static>;

    // TODO: add name as per spec?
    fn new() -> Self;

    fn insert(
        &mut self,
        priority: u32,
        ttl: Duration,
        message: Publication,
    ) -> Result<String, Error>;

    fn remove(&mut self, key: String) -> Result<bool, Error>;

    fn iter(&mut self, count: usize) -> Self::Loader;

    // TODO: what is the point of this func defined in the spec?
    // fn batch_iter(self, count: usize) -> MovingWindowIter<Self::Loader>;
}

// TODO: What is the point of this sliding window?
// MOVING WINDOW ITER
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

// TODO: should the iterator here be going over refs instead of values?
// TODO: are lifetimes correct
trait MessageLoader<'a> {
    type Iter: Iterator<Item = &'a (String, Publication)> + 'a;

    fn range(&'a self, count: u32) -> Self::Iter;
}

struct SimpleMessageLoader {
    messages: Vec<(String, Publication)>,
}

impl<'a> MessageLoader<'a> for SimpleMessageLoader {
    type Iter = Iter<'a, (String, Publication)>;

    // TODO: what is the expectation on this function? sliding window so need to store state about last call?
    fn range(&'a self, count: u32) -> Iter<'a, (String, Publication)> {
        self.messages[0..count as usize].iter()
    }
}

struct SimpleQueue {
    state: IndexMap<String, Publication>,
    offset: u32,
}

impl Queue for SimpleQueue {
    type Loader = SimpleMessageLoader;

    fn new() -> SimpleQueue {
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
        self.state
            .remove(&key)
            .ok_or(QueueError::RemovalFailure())?;

        Ok(true)
    }

    // TODO: do not clone
    fn iter(&mut self, count: usize) -> SimpleMessageLoader {
        let mut iter = self.state.iter();
        let mut output = vec![];

        let loop_count = 0;
        while let Some(pair) = iter.next() {
            if loop_count == count {
                break;
            }

            output.push((pair.0.clone(), pair.1.clone()));
        }

        SimpleMessageLoader { messages: output }
    }

    // fn batch_iter(self, count: usize) -> MovingWindowIter<Self::Loader> {}
}

#[derive(Debug, Error)]
enum QueueError {
    #[error("Failed to remove messages from queue")]
    RemovalFailure(),
}
