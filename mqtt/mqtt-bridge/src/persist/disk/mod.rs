use std::io::Error;
use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use mqtt3::proto::Publication;
use parking_lot::Mutex;
use tracing::debug;

use crate::persist::disk::loader::DiskMessageLoader;
use crate::persist::disk::waking_store::WakingStore;
use crate::persist::Key;
use crate::persist::Persist;

mod loader;
mod waking_store;

/// Disk persistence implementation used for the bridge
struct DiskPersist {
    waking_store: Arc<Mutex<WakingStore>>,
    offset: u32,
    loader: Arc<Mutex<DiskMessageLoader>>,
}

#[async_trait]
impl<'a> Persist<'a> for DiskPersist {
    type Loader = DiskMessageLoader;
    type Error = Error;

    async fn new(path: Path, batch_size: usize) -> Self {
        // make new waking store
        // set offset
        // make loader
        //return self
    }

    async fn push(&mut self, message: Publication) -> Result<Key, Self::Error> {
        // insert it into rocksdb
    }

    async fn remove(&mut self, key: Key) -> Option<Publication> {
        // remove it from the in-flight store if available
    }

    async fn loader(&'a mut self) -> Arc<Mutex<Self::Loader>> {
        // get a reference to the loader
    }
}

#[cfg(test)]
mod tests {}
