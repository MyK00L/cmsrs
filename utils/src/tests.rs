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
    let storage = storage::FsStorageHelper::new(&test_path);
    assert!(storage.is_ok());
    assert!(test_path.exists());
    assert!(test_path.is_dir());
}

#[test]
fn already_init_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let _ = storage::FsStorageHelper::new(&test_path);
    let storage2 = storage::FsStorageHelper::new(&test_path);
    assert!(storage2.is_ok());
}

#[test]
fn add_folder_to_root_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let res = storage::FsStorageHelper::new(&test_path).and_then(|fs| fs.add_folder("test2", None));
    assert!(res.is_ok());
    let unwrapped = res.unwrap();
    assert!(unwrapped.is_dir());
    assert_eq!(unwrapped, test_path.join("test2"));
}

#[test]
fn add_inner_folder_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let res = storage::FsStorageHelper::new(&test_path)
        .and_then(|fs| fs.add_folder("outer", None).map(|path| (fs, path)))
        .and_then(|(fs, path)| fs.add_folder("inner", Some(&path)));
    assert!(res.is_ok());
    let unwrapped = res.unwrap();
    assert!(unwrapped.is_dir());
    assert_eq!(unwrapped, test_path.join("outer").join("inner"));
}

#[test]
fn search_folder_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let res = storage::FsStorageHelper::new(&test_path)
        .and_then(|fs| fs.add_folder("outer", None).map(|path| (fs, path)))
        .and_then(|(fs, outer_path)| {
            fs.add_folder("inner", Some(&outer_path))
                .map(|inner_path| (fs, outer_path, inner_path))
        })
        .and_then(|(fs, outer_path, inner_path)| {
            fs.search_item(
                Some(&outer_path),
                inner_path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap(),
                None,
            )
        });
    assert!(res.is_ok());
    let unwrapped = res.unwrap();
    assert!(unwrapped.is_some());
    let unwrapped = unwrapped.unwrap();
    assert_eq!(unwrapped, test_path.join("outer").join("inner"));
}

#[test]
fn search_folder_not_found_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let res = storage::FsStorageHelper::new(&test_path)
        .and_then(|fs| fs.add_folder("outer", None).map(|path| (fs, path)))
        .and_then(|(fs, outer_path)| {
            fs.add_folder("inner", Some(&outer_path))
                .map(|inner_path| (fs, outer_path, inner_path))
        })
        .and_then(|(fs, outer_path, _)| fs.search_item(Some(&outer_path), "notgood", None));
    assert!(res.is_ok());
    let unwrapped = res.unwrap();
    assert!(unwrapped.is_none());
}

#[test]
fn save_file_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let res = storage::FsStorageHelper::new(&test_path)
        .and_then(|fs| fs.save_file(None, "testfile", "txt", b"123"));
    assert!(res.is_ok());
    let unwrapped = res.unwrap();
    println!("{}", unwrapped.to_str().unwrap());
    assert_eq!(unwrapped, test_path.join("testfile.txt"));
}

#[test]
fn save_and_search_file_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let storage = storage::FsStorageHelper::new(&test_path).unwrap();
    let res = storage
        .save_file(None, "testfile", "txt", b"123")
        .and_then(|_| storage.search_item(None, "testfile", Some("txt")));
    assert!(res.is_ok());
    let unwrapped = res.unwrap();
    assert_eq!(unwrapped, Some(test_path.join("testfile.txt")));
}

#[test]
fn save_search_and_read_file_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let storage = storage::FsStorageHelper::new(&test_path).unwrap();
    let mut buffer = vec![];

    let res = storage
        .save_file(None, "testfile", "txt", b"123")
        .and_then(|_| storage.search_item(None, "testfile", Some("txt")))
        .and_then(|path| storage.read_file(&path.unwrap(), &mut buffer));
    assert!(res.is_ok());
    assert_eq!(buffer, b"123");
}

#[test]
fn add_folder_save_and_search_file_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let storage = storage::FsStorageHelper::new(&test_path).unwrap();
    let res = storage
        .add_folder("testfolder", None)
        .and_then(|path| {
            storage
                .save_file(Some(&path), "testfile", "txt", b"123")
                .map(|_| path)
        })
        .and_then(|path| storage.search_item(Some(&path), "testfile", Some("txt")));
    assert!(res.is_ok());
    let unwrapped = res.unwrap();
    assert_eq!(unwrapped, Some(test_path.join("testfolder/testfile.txt")));
}

#[test]
fn save_and_read_file_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let storage = storage::FsStorageHelper::new(&test_path).unwrap();
    let mut buffer = vec![];

    let res = storage
        .save_file(None, "testfile", "txt", b"123")
        .and_then(|path| storage.read_file(&path, &mut buffer));
    assert!(res.is_ok());
    assert_eq!(buffer, b"123");
}

#[test]
fn save_and_read_object_file_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let storage = storage::FsStorageHelper::new(&test_path).unwrap();
    let mut buffer = vec![];
    let object = "testobject";

    let res = storage
        .save_file_object(None, "testfile", "txt", object)
        .and_then(|path| storage.read_file_object::<&str>(&path, &mut buffer));
    assert!(res.is_ok());
    let unwrapped = res.unwrap();
    assert_eq!(unwrapped, object);
}
