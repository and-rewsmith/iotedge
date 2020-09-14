use std::task::Waker;

use anyhow::Result;
use async_trait::async_trait;
use mqtt3::proto::Publication;

use crate::persist::Key;
use crate::persist::PersistError;

pub mod waking_map;
pub mod waking_store;

// TODO REVIEW: add newlines to impl
#[async_trait]
pub trait StreamWakeableState {
    fn insert(&mut self, key: Key, value: Publication) -> Result<(), PersistError>;

    fn get(&mut self, count: usize) -> Vec<(Key, Publication)>;

    fn remove_in_flight(&mut self, key: &Key) -> Option<Publication>;

    fn set_waker(&mut self, waker: &Waker);
}

#[cfg(test)]
mod tests {
    use std::{pin::Pin, sync::Arc, task::Context, task::Poll};

    use bytes::Bytes;
    use futures_util::stream::{Stream, StreamExt};
    use matches::assert_matches;
    use mqtt3::proto::{Publication, QoS};
    use parking_lot::Mutex;
    use test_case::test_case;
    use tokio::sync::Notify;

    use crate::persist::test_util::init_disk_persist_state;
    use crate::persist::waking_state::StreamWakeableState;
    use crate::persist::{waking_state::waking_map::WakingMap, Key};

    #[test_case(init_disk_persist_state())]
    #[test_case(WakingMap::new())]
    fn insert(mut state: impl StreamWakeableState) {
        let key1 = Key { offset: 0 };
        let pub1 = Publication {
            topic_name: "test".to_string(),
            qos: QoS::ExactlyOnce,
            retain: true,
            payload: Bytes::new(),
        };

        state.insert(key1, pub1.clone()).unwrap();

        let current_state = state.get(1);
        let extracted_message = current_state.get(0).unwrap().1.clone();
        assert_eq!(pub1, extracted_message);
    }

    // fn get_over_quantity(state: impl StreamWakeableState) {
    //     let mut state = WakingMap::new();

    //     let key1 = Key { offset: 0 };
    //     let pub1 = Publication {
    //         topic_name: "test".to_string(),
    //         qos: QoS::ExactlyOnce,
    //         retain: true,
    //         payload: Bytes::new(),
    //     };

    //     state.insert(key1, pub1.clone()).unwrap();

    //     let too_many_elements = 20;
    //     let current_state = state.get(too_many_elements);
    //     assert_eq!(current_state.len(), 1);

    //     let extracted_message = current_state.get(0).unwrap().1.clone();
    //     assert_eq!(pub1, extracted_message);
    // }

    // #[test]
    // fn in_flight() {
    //     let mut state = WakingMap::new();

    //     let key1 = Key { offset: 0 };
    //     let pub1 = Publication {
    //         topic_name: "test".to_string(),
    //         qos: QoS::ExactlyOnce,
    //         retain: true,
    //         payload: Bytes::new(),
    //     };

    //     assert_eq!(state.in_flight.len(), 0);
    //     state.insert(key1.clone(), pub1.clone()).unwrap();
    //     assert_eq!(state.in_flight.len(), 0);

    //     state.get(1);
    //     assert_eq!(state.in_flight.len(), 1);

    //     let removed = state.remove_in_flight(&key1).unwrap();
    //     assert_eq!(removed, pub1);
    //     assert_eq!(state.in_flight.len(), 0);
    // }

    // #[test]
    // fn remove_in_flight_dne() {
    //     let mut state = WakingMap::new();

    //     let key1 = Key { offset: 0 };
    //     let pub1 = Publication {
    //         topic_name: "test".to_string(),
    //         qos: QoS::ExactlyOnce,
    //         retain: true,
    //         payload: Bytes::new(),
    //     };

    //     state.insert(key1.clone(), pub1).unwrap();
    //     let bad_removal = state.remove_in_flight(&key1);
    //     assert_matches!(bad_removal, None);
    // }

    // #[tokio::test]
    // async fn insert_wakes_stream() {
    //     // setup data
    //     let key1 = Key { offset: 0 };
    //     let pub1 = Publication {
    //         topic_name: "test".to_string(),
    //         qos: QoS::ExactlyOnce,
    //         retain: true,
    //         payload: Bytes::new(),
    //     };

    //     let state = WakingMap::new();
    //     let state = Arc::new(Mutex::new(state));

    //     // start reading stream in a separate thread
    //     // this stream will return pending until woken up
    //     let map_copy = Arc::clone(&state);
    //     let notify = Arc::new(Notify::new());
    //     let notify2 = notify.clone();
    //     let poll_stream = async move {
    //         let mut test_stream = TestStream::new(map_copy, notify2);
    //         assert_eq!(test_stream.next().await.unwrap(), 1);
    //     };

    //     let poll_stream_handle = tokio::spawn(poll_stream);
    //     notify.notified().await;

    //     // insert an element to wake the stream, then wait for the other thread to complete
    //     state.lock().insert(key1, pub1).unwrap();
    //     poll_stream_handle.await.unwrap();
    // }

    // struct TestStream {
    //     waking_map: Arc<Mutex<WakingMap>>,
    //     notify: Arc<Notify>,
    //     should_return_pending: bool,
    // }

    // impl TestStream {
    //     fn new(waking_map: Arc<Mutex<WakingMap>>, notify: Arc<Notify>) -> Self {
    //         TestStream {
    //             waking_map,
    //             notify,
    //             should_return_pending: true,
    //         }
    //     }
    // }

    // impl Stream for TestStream {
    //     type Item = u32;

    //     fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    //         let mut_self = self.get_mut();
    //         let mut map_lock = mut_self.waking_map.lock();

    //         if mut_self.should_return_pending {
    //             mut_self.should_return_pending = false;
    //             map_lock.set_waker(cx.waker());
    //             mut_self.notify.notify();
    //             Poll::Pending
    //         } else {
    //             Poll::Ready(Some(1))
    //         }
    //     }
    // }
}
