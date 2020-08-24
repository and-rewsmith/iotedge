use std::{iter::Iterator, time::Duration};

use anyhow::Error;
use anyhow::Result;
use mqtt3::proto::Publication;
use thiserror::Error;

mod simple_message_loader;
mod simple_queue;

// TODO: are these lifetimes correct?
trait Queue {
    type Loader: MessageLoader<'static>;

    // TODO: add name as per spec?
    fn new() -> Self;

    // TODO: futureproof make return key type a struct that takes all req fields but also convert to string
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

// TODO: should the iterator here be going over refs instead of values?
// TODO: are lifetimes correct
trait MessageLoader<'a> {
    type Iter: Iterator<Item = &'a (String, Publication)> + 'a;

    // TODO: change to keys
    fn range(&'a self, count: usize) -> Result<Self::Iter>;
}

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("Failed to remove messages from queue")]
    Removal(),

    #[error("Failed loading message from queue")]
    LoadMessage(),
}

// TODO: stream
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
