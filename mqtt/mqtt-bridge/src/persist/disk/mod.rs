use std::io::Error;
use std::sync::Arc;
use std::Path;

use anyhow::Result;
use async_trait::async_trait;
use mqtt3::proto::Publication;
use parking_lot::Mutex;
use rocksdb::DB;
use tracing::debug;

use crate::persist::disk::loader::DiskMessageLoader;
use crate::persist::Key;
use crate::persist::Persist;

mod loader;

/// In memory persistence implementation used for the bridge
struct DiskPersist {}

#[async_trait]
impl<'a> Persist<'a> for DiskPersist {
    type Loader = DiskMessageLoader;
    type Error = Error;

    async fn new(path: Path, batch_size: usize) -> Self {}

    async fn push(&mut self, message: Publication) -> Result<Key, Self::Error> {}

    async fn remove(&mut self, key: Key) -> Option<Publication> {}

    async fn loader(&'a mut self) -> Arc<Mutex<Self::Loader>> {}
}

#[cfg(test)]
mod tests {}
