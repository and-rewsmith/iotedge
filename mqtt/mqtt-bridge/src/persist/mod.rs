use std::task::Waker;

use anyhow::Result;
use async_trait::async_trait;
use bincode::ErrorKind;
use mqtt3::proto::Publication;
use rocksdb::Error;
use rocksdb::DB;
use serde::{Deserialize, Serialize};
use thiserror::Error;

mod disk;
pub mod loader;
mod memory;
pub mod persistor;

#[async_trait]
pub trait StreamWakeableState {
    fn new(path: DB) -> Self;

    fn insert(&mut self, key: Key, value: Publication) -> Result<(), PersistError>;

    /// Get count elements of store, exluding those that are already in in-flight
    fn get(&mut self, count: usize) -> Vec<(Key, Publication)>;

    fn remove_in_flight(&mut self, key: &Key) -> Option<Publication>;

    fn set_waker(&mut self, waker: &Waker);
}

/// Keys used in persistence.
/// Ordered by offset
#[derive(Hash, Eq, Ord, PartialOrd, PartialEq, Clone, Debug, Deserialize, Serialize)]
pub struct Key {
    offset: u32,
}

#[derive(Debug, Error)]
pub enum PersistError {
    #[error("Failed to serialize on database insert")]
    Serialization(#[from] Box<ErrorKind>),

    #[error("Failed to serialize on database insert")]
    Insertion(#[from] Error),
}

#[cfg(test)]
mod tests {
    use crate::persist::Key;

    #[test]
    fn key_offset_ordering() {
        // ordered by offset
        let key1 = Key { offset: 0 };
        let key2 = Key { offset: 1 };
        let key3 = Key { offset: 1 };
        assert!(key2 > key1);
        assert!(key2 != key1);
        assert!(key1 < key2);
        assert!(key2 == key3);
    }
}
