#![allow(dead_code)] // TODO remove when ready

use std::{collections::hash_map::Entry, collections::HashMap, task::Waker};

use bincode::{self};
use mqtt3::proto::Publication;
use rocksdb::{IteratorMode, Options, DB};
use uuid::Uuid;

use crate::{
    persist::{waking_state::StreamWakeableState, Key, PersistError},
    settings::PersistenceSettings,
};

// defaults taken from edgehub core RocksDbOptionsProvider
const WRITE_BUFFER_SIZE: usize = 2 * 1024 * 1024;
const TARGET_FILE_SIZE_BASE: u64 = 2 * 1024 * 1024;
const MAX_BYTES_FOR_LEVEL_BASE: u64 = 10 * 1024 * 1024;
const SOFT_PENDING_COMPACTION_BYTES: usize = 10 * 1024 * 1024;
const HARD_PENDING_COMPACTION_BYTES: usize = 1024 * 1024 * 1024;

/// When elements are retrieved they are added to the in flight collection, but kept in the original db store.
/// Only when elements are removed from the in-flight collection they will be removed from the store.
pub struct WakingStore {
    db: DB,
    in_flight: HashMap<Key, Publication>,
    waker: Option<Waker>,
    column_family: String,
}

impl WakingStore {
    pub fn new(mut db: DB, settings: &PersistenceSettings) -> Result<Self, PersistError> {
        let column_family = Uuid::new_v4().to_string();
        let mut options = Options::default();
        options.set_max_open_files(settings.max_open_files());
        options.set_max_total_wal_size(settings.max_wal_size());
        // TODO REVIEW: storage log level? Log level files to 0, but then we will get runtime errors :(

        if !settings.optimize_for_performance() {
            options.set_write_buffer_size(WRITE_BUFFER_SIZE);
            options.set_target_file_size_base(TARGET_FILE_SIZE_BASE);
            options.set_max_bytes_for_level_base(MAX_BYTES_FOR_LEVEL_BASE);
            options.set_soft_pending_compaction_bytes_limit(SOFT_PENDING_COMPACTION_BYTES);
            options.set_hard_pending_compaction_bytes_limit(HARD_PENDING_COMPACTION_BYTES);
        }

        db.create_cf(column_family.clone(), &options)
            .map_err(PersistError::CreateColumnFamily)?;

        Ok(Self {
            db,
            in_flight: HashMap::new(),
            waker: None,
            column_family,
        })
    }
}

impl StreamWakeableState for WakingStore {
    fn insert(&mut self, key: Key, value: Publication) -> Result<(), PersistError> {
        let key_bytes = bincode::serialize(&key).map_err(PersistError::Serialization)?;
        let publication_bytes = bincode::serialize(&value).map_err(PersistError::Serialization)?;

        let column_family = self
            .db
            .cf_handle(&self.column_family)
            .ok_or(PersistError::GetColumnFamily())?;
        self.db
            .put_cf(column_family, key_bytes, publication_bytes)
            .map_err(PersistError::Insertion)?;

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }

        Ok(())
    }

    /// Get count elements of store, exluding those that are already in in-flight
    fn batch(&mut self, count: usize) -> Result<Vec<(Key, Publication)>, PersistError> {
        let column_family = self
            .db
            .cf_handle(&self.column_family)
            .ok_or(PersistError::GetColumnFamily())?;
        let iter = self.db.iterator_cf(column_family, IteratorMode::Start);

        let mut output = vec![];
        for (iterations, extracted) in iter.enumerate() {
            let (key, publication) = bincode::deserialize(&*extracted.0)
                .map_err(PersistError::Deserialization)
                .and_then(|key: Key| {
                    bincode::deserialize(&extracted.1)
                        .map_err(PersistError::Deserialization)
                        .map(|publication: Publication| (key, publication))
                })?;

            if let Entry::Vacant(o) = self.in_flight.entry(key.clone()) {
                o.insert(publication.clone());
                output.push((key.clone(), publication.clone()));
                self.in_flight.insert(key, publication);
            }

            if iterations == count {
                break;
            }
        }

        Ok(output)
    }

    fn remove_in_flight(&mut self, key: &Key) -> Result<Publication, PersistError> {
        let key_bytes = bincode::serialize(&key).map_err(PersistError::Serialization)?;

        let column_family = self
            .db
            .cf_handle(&self.column_family)
            .ok_or(PersistError::GetColumnFamily())?;

        self.db
            .delete_cf(column_family, key_bytes)
            .map_err(PersistError::Removal)?;
        let removed = self
            .in_flight
            .remove(key)
            .ok_or(PersistError::RemovalForMissing())?;
        Ok(removed)
    }

    fn set_waker(&mut self, waker: &Waker) {
        self.waker = Some(waker.clone());
    }
}
