use std::fs;
use std::path::Path;

use rocksdb::DB;

use crate::persist::waking_state::waking_store::WakingStore;

const STORAGE_DIR: &str = "unit-tests/persistence/";

pub fn init_disk_persist_state() -> WakingStore {
    clear_test_persist_folder();

    let db = DB::open_default(STORAGE_DIR).unwrap();
    WakingStore::new(db)
}

fn clear_test_persist_folder() {
    let path = Path::new(STORAGE_DIR);
    let storage_dir_root = path.components().next().unwrap();
    if let Err(_) = fs::remove_dir_all(storage_dir_root) {
        ()
    }
}
