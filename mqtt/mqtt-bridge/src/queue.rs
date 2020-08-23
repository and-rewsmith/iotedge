use std::{iter::Iterator, time::Duration};

use anyhow::Error;
use mqtt3::proto::Publication;

mod simple_queue;

// TODO: are these lifetimes correct?
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
