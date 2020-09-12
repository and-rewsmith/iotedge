use std::sync::Arc;

use anyhow::Result;
use mqtt3::proto::Publication;
use parking_lot::Mutex;
use tracing::debug;

use crate::persist::loader::MessageLoader;
use crate::persist::waking_state::StreamWakeableState;
use crate::persist::Key;
use crate::persist::PersistError;

/// Persistence implementation used for the bridge
pub struct Persistor<S: StreamWakeableState> {
    state: Arc<Mutex<S>>,
    offset: u32,
    loader: Arc<Mutex<MessageLoader<S>>>,
}

impl<S: StreamWakeableState> Persistor<S> {
    async fn new(state: S, batch_size: usize) -> Self {
        let state = Arc::new(Mutex::new(state));
        let loader = MessageLoader::new(Arc::clone(&state), batch_size).await;
        let loader = Arc::new(Mutex::new(loader));

        let offset = 0;
        Persistor {
            state,
            offset,
            loader,
        }
    }

    async fn push(&mut self, message: Publication) -> Result<Key, PersistError> {
        debug!(
            "persisting publication on topic {} with offset {}",
            message.topic_name, self.offset
        );

        let key = Key {
            offset: self.offset,
        };

        let mut state_lock = self.state.lock();
        state_lock.insert(key.clone(), message)?;
        self.offset += 1;
        Ok(key)
    }

    async fn remove(&mut self, key: Key) -> Option<Publication> {
        debug!(
            "removing publication with offset {} from in-flight collection",
            self.offset
        );

        let mut state_lock = self.state.lock();
        state_lock.remove_in_flight(&key)
    }

    async fn loader(&mut self) -> Arc<Mutex<MessageLoader<S>>> {
        Arc::clone(&self.loader)
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use futures_util::stream::StreamExt;
    use matches::assert_matches;
    use mqtt3::proto::{Publication, QoS};

    use crate::persist::test_util::init_disk_persist_state;
    use crate::persist::waking_state::waking_map::WakingMap;
    use crate::persist::waking_state::StreamWakeableState;
    use crate::persist::{persistor::Persistor, Key};

    #[tokio::test]
    async fn insert_memory() {
        let state = WakingMap::new();
        insert(state);
    }

    #[tokio::test]
    async fn insert_disk() {
        let state = init_disk_persist_state();
        insert(state);
    }

    async fn insert(state: impl StreamWakeableState) {
        // setup state
        let batch_size: usize = 5;
        let mut persistence = Persistor::new(state, batch_size).await;

        // setup data
        let key1 = Key { offset: 0 };
        let key2 = Key { offset: 1 };
        let pub1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };
        let pub2 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        // insert some elements
        persistence.push(pub1.clone()).await.unwrap();
        persistence.push(pub2.clone()).await.unwrap();

        // get loader
        let loader = persistence.loader().await;
        let mut loader = loader.lock();

        // make sure same publications come out in correct order
        let extracted1 = loader.next().await.unwrap();
        let extracted2 = loader.next().await.unwrap();
        assert_eq!(extracted1.0, key1);
        assert_eq!(extracted2.0, key2);
        assert_eq!(extracted1.1, pub1);
        assert_eq!(extracted2.1, pub2);
    }

    #[tokio::test]
    async fn remove_memory() {
        let state = WakingMap::new();
        remove(state);
    }

    #[tokio::test]
    async fn remove_disk() {
        let state = init_disk_persist_state();
        remove(state);
    }

    async fn remove(state: impl StreamWakeableState) {
        // setup state
        let batch_size: usize = 1;
        let mut persistence = Persistor::new(state, batch_size).await;

        // setup data
        let key1 = Key { offset: 0 };
        let key2 = Key { offset: 1 };
        let pub1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };
        let pub2 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        // insert some elements
        persistence.push(pub1.clone()).await.unwrap();

        // get loader
        let loader = persistence.loader().await;
        let mut loader = loader.lock();

        // TODO REVIEW: verify removal returns correct element
        // process first message, forcing loader to get new batch on the next read
        loader.next().await.unwrap();
        persistence.remove(key1).await.unwrap();

        // add a second message and verify this is returned first by loader
        persistence.push(pub2.clone()).await.unwrap();
        let extracted = loader.next().await.unwrap();
        assert_eq!((extracted.0, extracted.1), (key2, pub2));
    }

    #[tokio::test]
    async fn remove_key_dne_memory() {
        let state = WakingMap::new();
        remove_key_dne(state);
    }

    #[tokio::test]
    async fn remove_key_dne_disk() {
        let state = init_disk_persist_state();
        remove_key_dne(state);
    }

    async fn remove_key_dne(state: impl StreamWakeableState) {
        // setup state
        let batch_size: usize = 1;
        let mut persistence = Persistor::new(state, batch_size).await;

        // setup data
        let key1 = Key { offset: 0 };

        // verify failed removal
        let removal = persistence.remove(key1).await;
        assert_matches!(removal, None);
    }

    #[tokio::test]
    async fn get_loader_memory() {
        let state = WakingMap::new();
        get_loader(state);
    }

    #[tokio::test]
    async fn get_loader_disk() {
        let state = init_disk_persist_state();
        get_loader(state);
    }

    async fn get_loader(state: impl StreamWakeableState) {
        // setup state
        let batch_size: usize = 1;
        let mut persistence = Persistor::new(state, batch_size).await;

        // setup data
        let key1 = Key { offset: 0 };
        let key2 = Key { offset: 1 };
        let pub1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };
        let pub2 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        // insert 2 elements
        persistence.push(pub1.clone()).await.unwrap();
        persistence.push(pub2.clone()).await.unwrap();

        // get loader with batch size
        let loader = persistence.loader().await;
        let mut loader = loader.lock();

        // verify the loader returns both elements
        let extracted1 = loader.next().await.unwrap();
        let extracted2 = loader.next().await.unwrap();
        assert_eq!(extracted1.0, key1);
        assert_eq!(extracted2.0, key2);
        assert_eq!(extracted1.1, pub1);
        assert_eq!(extracted2.1, pub2);
    }
}