use super::*;

use temp_testdir::TempDir;

fn get_test_time() -> std::time::SystemTime {
    std::time::UNIX_EPOCH
        + std::time::Duration::from_secs(10)
        + std::time::Duration::from_nanos(101)
}
#[test]
fn convert_to_mongo_timestamp_and_back_test() {
    let now = std::time::UNIX_EPOCH
        + std::time::Duration::from_secs(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        ); //This is needed because we need to discard anything below the second
    let mongo_now = mongo::systime_to_timestamp(now);
    assert_eq!(now, mongo::timestamp_to_systime(mongo_now));
}

#[test]
fn convert_to_mongo_timestamp_test() {
    let test_time = get_test_time();
    let mongo_test_time = mongo::systime_to_timestamp(test_time);
    assert_eq!(mongo_test_time.time, 10);
    assert_eq!(mongo_test_time.increment, 0);
}

#[test]
fn convert_from_mongo_timestamp_test() {
    let test_time = get_test_time() - std::time::Duration::from_nanos(101);
    let mongo_test_time = bson::Timestamp {
        time: 10,
        increment: 0,
    };
    assert_eq!(mongo::timestamp_to_systime(mongo_test_time), test_time);
}

#[test]
fn init_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let storage = storage::FsStorageHelper::new(test_path.as_path());
    assert!(storage.is_ok());
    assert!(test_path.exists());
    assert!(test_path.is_dir());
}

#[test]
fn already_init_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let _ = storage::FsStorageHelper::new(test_path.as_path());
    let storage2 = storage::FsStorageHelper::new(test_path.as_path());
    assert!(storage2.is_ok());
}

#[test]
fn add_storage_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let test_container_path = test_path.join("testcontaner");
    let res = storage::FsStorageHelper::new(test_path.as_path())
        .and_then(|store| store.add_container(test_container_path.as_path()));
    assert!(res.is_ok());
    assert!(test_container_path.exists());
}

#[test]
fn add_storage_recursive_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let test_container_recursive_path = test_path.join("foo/bar/asd/testcontaner");
    let res = storage::FsStorageHelper::new(test_path.as_path())
        .and_then(|store| store.add_container(test_container_recursive_path.as_path()));
    assert!(res.is_ok());
    assert!(test_container_recursive_path.exists());
}

#[test]
fn already_add_storage_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let test_container_path = test_path.join("testcontaner");
    let res = storage::FsStorageHelper::new(test_path.as_path())
        .and_then(|store| store.add_container(test_container_path.as_path()))
        .and_then(|store| store.add_container(test_container_path.as_path()));
    assert!(res.is_ok());
}
