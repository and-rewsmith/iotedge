use std::task::Waker;

use anyhow::Result;
use async_trait::async_trait;
use mqtt3::proto::Publication;

use crate::persist::Key;
use crate::persist::PersistError;

pub mod waking_map;
pub mod waking_store;

#[async_trait]
pub trait StreamWakeableState {
    fn insert(&mut self, key: Key, value: Publication) -> Result<(), PersistError>;

    /// Get count elements of store, exluding those that are already in in-flight
    fn get(&mut self, count: usize) -> Vec<(Key, Publication)>;

    fn remove_in_flight(&mut self, key: &Key) -> Option<Publication>;

    fn set_waker(&mut self, waker: &Waker);
}
