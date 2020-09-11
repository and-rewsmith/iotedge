use std::boxed::Box;
use std::fs;
use std::path::Path;

use rocksdb::DB;

use crate::persist::disk::WakingStore;
use crate::persist::memory::WakingMap;
use crate::persist::StreamWakeableState;

const STORAGE_DIR: &str = "unit-tests/persistence/";

pub fn clear_test_persist_folder() {
    let path = Path::new(STORAGE_DIR);
    let storage_dir_root = path.components().next().unwrap();
    if let Err(_) = fs::remove_dir_all(storage_dir_root) {
        ()
    }
}

pub fn create_test_db() -> DB {
    DB::open_default(STORAGE_DIR).unwrap()
}

pub fn stream_wakeable_states() -> Vec<Box<dyn StreamWakeableState>> {
    let state1 = WakingMap::new();

    let db = create_test_db();
    let state2 = WakingStore::new(db);

    let output: Vec<Box<dyn StreamWakeableState>> = vec![Box::new(state1), Box::new(state2)];
    output
}
