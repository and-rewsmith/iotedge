use std::iter::Iterator;
use std::time::Duration;

use anyhow::Error;
use mqtt3::proto::Publication;

trait Queue {
    type Loader: MessageLoader;

    // TODO: name?
    fn new(name: String);

    fn insert(priority: u32, ttl: Duration, message: Publication) -> Result<String, Error>;

    fn remove(key: String) -> Result<bool, Error>;

    fn batch_iter(count: usize) -> MovingWindowIter<Self::Loader>;
}

trait MessageLoader {
    type Iter: Iterator<Item = (String, Publication)>;

    fn range(&self, count: u32) -> Self::Iter;
}

struct MovingWindowIter<L>
where
    L: MessageLoader,
{
    count: usize,

    messages: L,
}
