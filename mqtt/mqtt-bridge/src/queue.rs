use std::ops::RangeBounds;
use std::{iter::Iterator, time::Duration};

use anyhow::Error;
use anyhow::Result;
use futures_util::stream::Stream;
use mqtt3::proto::Publication;
use thiserror::Error;

mod simple_message_loader;
mod simple_queue;

// TODO: are these lifetimes correct?
trait Queue {
    type Loader: Stream<'static>;

    // TODO: add name as per spec?
    fn new() -> Self;

    // TODO: futureproof make return key type a struct that takes all req fields but also convert to string
    fn insert(&mut self, priority: u32, ttl: Duration, message: Publication) -> Result<Key, Error>;

    fn remove(&mut self, key: Key) -> Result<bool, Error>;

    fn get_loader(&mut self, batch_size: usize) -> Self::Loader;
}

// TODO: we will have a heap sorted by these keys so we will have to implement some comparaotr
struct Key {
    offset: u32,
    priority: u32,
    ttl: Duration,
}

// trait MessageLoader<'a> {
//     type Iter: Iterator<Item = (&'a String, &'a Publication)> + 'a;

//     // TODO: change to keys
//     fn range(&'a self, keys: impl RangeBounds<(String, Publication)>) -> Result<Self::Iter>;
// }

#[derive(Debug, Error)]
pub enum QueueError {
    #[error("Failed to remove messages from queue")]
    Removal(),

    #[error("Failed loading message from queue")]
    LoadMessage(),
}
