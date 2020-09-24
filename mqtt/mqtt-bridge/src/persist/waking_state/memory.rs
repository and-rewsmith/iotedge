use std::{
    cmp::min,
    collections::{HashSet, VecDeque},
    task::Waker,
};

use mqtt3::proto::Publication;

use crate::persist::{waking_state::StreamWakeableState, Key, PersistError};

/// When elements are retrieved they are moved to the loaded collection.
pub struct WakingMemoryStore {
    queue: VecDeque<(Key, Publication)>,
    loaded: HashSet<Key>,
    waker: Option<Waker>,
}

impl WakingMemoryStore {
    pub fn new() -> Self {
        WakingMemoryStore {
            queue: VecDeque::new(),
            loaded: HashSet::new(),
            waker: None,
        }
    }
}

impl StreamWakeableState for WakingMemoryStore {
    fn insert(&mut self, key: Key, value: Publication) -> Result<(), PersistError> {
        self.queue.push_back((key, value));

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }

        Ok(())
    }

    fn batch(&mut self, count: usize) -> Result<VecDeque<(Key, Publication)>, PersistError> {
        let count = min(count, self.queue.len());
        let output: VecDeque<_> = self.queue.drain(..count).collect();

        for (key, _) in output.iter() {
            self.loaded.insert(key.clone());
        }

        Ok(output)
    }

    fn remove(&mut self, key: Key) -> Result<(), PersistError> {
        if self.loaded.remove(&key) {
            Ok(())
        } else {
            Err(PersistError::RemovalForMissing)
        }
    }

    fn set_waker(&mut self, waker: &Waker) {
        self.waker = Some(waker.clone());
    }
}