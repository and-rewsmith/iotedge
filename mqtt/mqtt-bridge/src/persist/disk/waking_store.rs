use std::{collections::HashMap, path::Path, task::Waker};

use bincode::deserialize;
use bincode::serialize;
use mqtt3::proto::Publication;
use rocksdb::Error;
use rocksdb::IteratorMode;
use rocksdb::DB;
// use rocksdb::{ColumnFamilyDescriptor, Options};
use thiserror::Error;
use tracing::error;

use crate::persist::Key;

// TODO: add interface?
// TODO: This will allow us to only use one loader.
// TODO: Will this also allow us only one persistor?

/// Responsible for waking waiting streams when new elements are added
/// Exposes a get method for retrieving a count of elements starting from queue head
/// When elements are retrieved via get they are added to the in flight collection
/// When elements are removed from the in-flight collection they will be removed from the store.
pub struct WakingStore {
    db: DB,
    in_flight: HashMap<Key, Publication>,
    waker: Option<Waker>,
}

impl WakingStore {
    pub fn new(path: &Path) -> Result<Self, WakingStoreError> {
        let db = DB::open_default(path).map_err(WakingStoreError::CreateDB)?;
        let in_flight = HashMap::new();
        let waker = None;

        Ok(Self {
            db,
            in_flight,
            waker,
        })
    }

    pub fn insert(&mut self, key: Key, value: Publication) -> Result<(), WakingStoreError> {
        let key_bytes = serialize(&key).unwrap();
        let publication_bytes = serialize(&value).unwrap();
        self.db.put(key_bytes, publication_bytes).unwrap();

        if let Some(waker) = self.waker.take() {
            waker.wake();
        }

        Ok(())
    }

    /// Get count elements of store, exluding those that are already in in-flight
    pub fn get(&mut self, count: usize) -> Vec<(Key, Publication)> {
        let mut iter = self.db.iterator(IteratorMode::Start);
        let mut output = vec![];

        let mut iterations = 0;
        while let Some(extracted) = iter.next() {
            let key: Key = deserialize(&*extracted.0).unwrap();
            let publication: Publication = deserialize(&*extracted.1).unwrap();

            if !self.in_flight.contains_key(&key) {
                output.push((key.clone(), publication.clone()));
                self.in_flight.insert(key, publication);
            }

            iterations += 1;
            if iterations == count {
                break;
            }
        }

        output
    }

    pub fn remove_in_flight(&mut self, key: &Key) -> Option<Publication> {
        let key_bytes = serialize(&key).unwrap();
        self.db.delete(key_bytes).unwrap();
        self.in_flight.remove(key)
    }

    pub fn set_waker(&mut self, waker: &Waker) {
        self.waker = Some(waker.clone());
    }
}

#[derive(Debug, Error)]
pub enum WakingStoreError {
    #[error("Failed to create database for on-disk persistent store")]
    CreateDB(#[from] Error),
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use std::{pin::Pin, sync::Arc, task::Context, task::Poll};

    use bytes::Bytes;
    use futures_util::stream::{Stream, StreamExt};
    use matches::assert_matches;
    use mqtt3::proto::{Publication, QoS};
    use parking_lot::Mutex;
    use serial_test::serial;
    use tokio::sync::Notify;

    use crate::persist::disk::waking_store::WakingStore;
    use crate::persist::Key;

    const STORAGE_DIR: &str = "unit-tests/persistence/";

    #[test]
    #[serial(persist)]
    fn insert() {
        clear_test_persist_folder();

        let path = Path::new(STORAGE_DIR);
        let mut state = WakingStore::new(path).unwrap();

        let key1 = Key { offset: 0 };
        let pub1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        state.insert(key1.clone(), pub1.clone()).unwrap();

        let current_state = state.get(1);
        let extracted_message = current_state.get(0).unwrap().1.clone();
        assert_eq!(pub1, extracted_message);

        clear_test_persist_folder();
    }

    #[test]
    #[serial(persist)]
    fn get_over_quantity_succeeds() {
        clear_test_persist_folder();

        let path = Path::new(STORAGE_DIR);
        let mut state = WakingStore::new(path).unwrap();

        let key1 = Key { offset: 0 };
        let pub1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        state.insert(key1.clone(), pub1.clone());

        let too_many_elements = 20;
        let current_state = state.get(too_many_elements);
        assert_eq!(current_state.len(), 1);

        let extracted_message = current_state.get(0).unwrap().1.clone();
        assert_eq!(pub1, extracted_message);

        clear_test_persist_folder();
    }

    #[test]
    #[serial(persist)]
    fn in_flight() {
        clear_test_persist_folder();

        let path = Path::new(STORAGE_DIR);
        let mut state = WakingStore::new(path).unwrap();

        let key1 = Key { offset: 0 };
        let pub1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        assert_eq!(state.in_flight.len(), 0);
        state.insert(key1.clone(), pub1.clone());
        assert_eq!(state.in_flight.len(), 0);

        state.get(1);
        assert_eq!(state.in_flight.len(), 1);

        let removed = state.remove_in_flight(&key1).unwrap();
        assert_eq!(removed, pub1);
        assert_eq!(state.in_flight.len(), 0);

        clear_test_persist_folder();
    }

    #[test]
    #[serial(persist)]
    fn remove_in_flight_dne() {
        clear_test_persist_folder();

        let path = Path::new(STORAGE_DIR);
        let mut state = WakingStore::new(path).unwrap();

        let key1 = Key { offset: 0 };
        let pub1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        state.insert(key1.clone(), pub1.clone());
        let bad_removal = state.remove_in_flight(&key1);
        assert_matches!(bad_removal, None);

        clear_test_persist_folder();
    }

    #[tokio::test]
    #[serial(persist)]
    async fn insert_wakes_stream() {
        clear_test_persist_folder();

        let path = Path::new(STORAGE_DIR);
        let state = WakingStore::new(path).unwrap();
        let state = Arc::new(Mutex::new(state));

        // setup data
        let key1 = Key { offset: 0 };
        let pub1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        // start reading stream in a separate thread
        // this stream will return pending until woken up
        let map_copy = Arc::clone(&state);
        let notify = Arc::new(Notify::new());
        let notify2 = notify.clone();
        let poll_stream = async move {
            let mut test_stream = TestStream::new(map_copy, notify2);
            assert_eq!(test_stream.next().await.unwrap(), 1);
        };

        let poll_stream_handle = tokio::spawn(poll_stream);
        notify.notified().await;

        // make sure waker is set
        let mut state_lock = state.lock();
        assert_matches!(state_lock.waker, Some(_));

        // insert an element to wake the stream, then wait for the other thread to complete
        state_lock.insert(key1, pub1);

        // make sure waker is removed
        assert_matches!(state_lock.waker, None);
        drop(state_lock);

        // make sure stream gets woke
        poll_stream_handle.await.unwrap();

        clear_test_persist_folder();
    }

    struct TestStream {
        waking_map: Arc<Mutex<WakingStore>>,
        notify: Arc<Notify>,
        should_return_pending: bool,
    }

    impl TestStream {
        fn new(waking_map: Arc<Mutex<WakingStore>>, notify: Arc<Notify>) -> Self {
            TestStream {
                waking_map,
                notify,
                should_return_pending: true,
            }
        }
    }

    impl Stream for TestStream {
        type Item = u32;

        fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
            let mut_self = self.get_mut();
            let mut map_lock = mut_self.waking_map.lock();

            if mut_self.should_return_pending {
                mut_self.should_return_pending = false;
                map_lock.set_waker(cx.waker());
                mut_self.notify.notify();
                Poll::Pending
            } else {
                Poll::Ready(Some(1))
            }
        }
    }

    fn clear_test_persist_folder() {
        let path = Path::new(STORAGE_DIR);
        let storage_dir_root = path.components().next().unwrap();
        fs::remove_dir_all(storage_dir_root);
    }
}
