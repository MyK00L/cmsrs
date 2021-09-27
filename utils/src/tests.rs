use super::*;

use crate::scoring::{calc_submission_score, calc_subtask_score};
use protos::common::{Duration, Resources, Score};
use protos::evaluation::*;
use protos::evaluation::{SubtaskResult, TestcaseResult};
use protos::scoring::{subtask, Subtask};
use temp_testdir::TempDir;

fn partial_score(correct: f64, total: f64) -> f64 {
    correct / total
}

fn get_test_time() -> std::time::SystemTime {
    std::time::UNIX_EPOCH
        + std::time::Duration::from_secs(10)
        + std::time::Duration::from_nanos(101)
}

const TESTCASE_AC_SCORE: Score = Score { score: 1.0 };
const WA_SCORE: Score = Score { score: 0.0 };

const MIN_SCORING_METHOD: Subtask = Subtask {
    method: subtask::Method::Min as i32,
    max_score: Score { score: 100.0 },
};

const SUM_SCORING_METHOD: Subtask = Subtask {
    method: subtask::Method::Sum as i32,
    max_score: Score { score: 100.0 },
};

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

    let res = storage
        .save_file(None, "testfile", "txt", b"123")
        .and_then(|_| storage.search_item(None, "testfile", Some("txt")))
        .and_then(|path| storage.read_file(&path.unwrap()));
    assert!(res.is_ok());
    let buffer = res.unwrap();
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

    let res = storage
        .save_file(None, "testfile", "txt", b"123")
        .and_then(|path| storage.read_file(&path));
    assert!(res.is_ok());
    let buffer = res.unwrap();
    assert_eq!(buffer, b"123");
}

#[test]
fn save_and_read_object_file_fs_storage_helper_test() {
    let temp_dir = TempDir::default();
    let test_path = temp_dir.to_path_buf().join("test");
    let storage = storage::FsStorageHelper::new(&test_path).unwrap();
    let object = String::from("testcontent");

    let res = storage
        .save_file_object(None, "testfile", "txt", object)
        .and_then(|path| storage.read_file_object::<String>(&path));
    assert!(res.is_ok());
    let unwrapped = res.unwrap();
    assert_eq!(unwrapped, "testcontent");
}

fn get_bool_testcase(result: bool) -> TestcaseResult {
    TestcaseResult {
        outcome: testcase_result::Outcome::Ok as i32,
        score: if result { TESTCASE_AC_SCORE } else { WA_SCORE },
        used_resources: Resources {
            time: Duration { secs: 0, nanos: 0 },
            memory_bytes: 0u64,
        },
        id: 1u64,
    }
}

/// Pre:
///    - result is in range \[0, 1\]
fn get_double_testcase(result: f64) -> TestcaseResult {
    assert!(!result.is_nan());
    TestcaseResult {
        outcome: testcase_result::Outcome::Ok as i32,
        score: Score { score: result },
        used_resources: Resources {
            time: Duration { secs: 0, nanos: 0 },
            memory_bytes: 0u64,
        },
        id: 1u64,
    }
}

#[test]
fn evaluate_wrong_bool_subtask_with_min_test() {
    let subtask_result_wrong = SubtaskResult {
        testcase_results: vec![
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(false),
            get_bool_testcase(true),
        ],
        score: WA_SCORE,
        id: 1u64,
    };

    let score = calc_subtask_score(&subtask_result_wrong.testcase_results, &MIN_SCORING_METHOD);
    assert_eq!(score, WA_SCORE);
}

#[test]
fn evaluate_correct_bool_subtask_with_min_test() {
    let subtask_result_correct = SubtaskResult {
        testcase_results: vec![
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(true),
        ],
        score: MIN_SCORING_METHOD.max_score,
        id: 1u64,
    };

    let score = calc_subtask_score(
        &subtask_result_correct.testcase_results,
        &MIN_SCORING_METHOD,
    );
    assert_eq!(score, subtask_result_correct.score);
}

#[test]
fn evaluate_wrong_bool_subtask_with_sum_test() {
    let subtask_result_wrong = SubtaskResult {
        testcase_results: vec![
            get_bool_testcase(false),
            get_bool_testcase(false),
            get_bool_testcase(false),
            get_bool_testcase(false),
            get_bool_testcase(false),
        ],
        score: WA_SCORE,
        id: 1u64,
    };

    let score = calc_subtask_score(&subtask_result_wrong.testcase_results, &SUM_SCORING_METHOD);
    assert_eq!(score, subtask_result_wrong.score);
}

#[test]
fn evaluate_partial_bool_subtask_with_sum_test() {
    let subtask_result_partial = SubtaskResult {
        testcase_results: vec![
            get_bool_testcase(false),
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(false),
            get_bool_testcase(true),
        ],
        score: Score {
            score: SUM_SCORING_METHOD.max_score.score * 3f64 / 5f64,
        },
        id: 1u64,
    };

    let score = calc_subtask_score(
        &subtask_result_partial.testcase_results,
        &SUM_SCORING_METHOD,
    );
    assert_eq!(score, subtask_result_partial.score);
}

#[test]
fn evaluate_correct_bool_subtask_with_sum_test() {
    let subtask_result_correct = SubtaskResult {
        testcase_results: vec![
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(true),
        ],
        score: SUM_SCORING_METHOD.max_score,
        id: 1u64,
    };

    let score = calc_subtask_score(
        &subtask_result_correct.testcase_results,
        &SUM_SCORING_METHOD,
    );
    assert_eq!(score, subtask_result_correct.score);
}

#[test]
fn evaluate_wrong_double_subtask_with_sum_test() {
    let subtask_result_wrong = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(0.0),
            get_double_testcase(0.0),
            get_double_testcase(0.0),
            get_double_testcase(0.0),
            get_double_testcase(0.0),
        ],
        score: Score { score: 0.0 },
        id: 1u64,
    };

    let score = calc_subtask_score(&subtask_result_wrong.testcase_results, &SUM_SCORING_METHOD);
    assert_eq!(score, subtask_result_wrong.score);
}
/*
#[test]
fn evaluate_partial_double_subtask_with_sum_test() {
    let num_of_testcases = 5f64;
    let subtask_result_partial = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(partial_score(2f64, 5f64) / num_of_testcases),
            get_double_testcase(partial_score(3f64, 5f64) / num_of_testcases),
            get_double_testcase(0.0),
            get_double_testcase(1.0 / num_of_testcases),
            get_double_testcase(partial_score(1f64, 5f64) / num_of_testcases),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(
                (SUM_SCORING_METHOD.max_score * DOUBLE_CORRECT_ANSWER
                    + partial_score(SUM_SCORING_METHOD.max_score * (2f64 + 3f64 + 1f64), 5f64))
                    / num_of_testcases,
            )),
        },
        id: 1u64,
    };

    let mut score = OneOfScore::default();
    evaluate_subtask_score(
        &subtask_result_partial.testcase_results,
        &SUM_SCORING_METHOD,
        &mut score,
    );
    assert_eq!(score, subtask_result_partial.score);
}

#[test]
fn evaluate_correct_double_subtask_with_sum_test() {
    let num_of_testcases = 5f64;
    let subtask_result_correct = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(DOUBLE_CORRECT_ANSWER / num_of_testcases),
            get_double_testcase(DOUBLE_CORRECT_ANSWER / num_of_testcases),
            get_double_testcase(DOUBLE_CORRECT_ANSWER / num_of_testcases),
            get_double_testcase(DOUBLE_CORRECT_ANSWER / num_of_testcases),
            get_double_testcase(DOUBLE_CORRECT_ANSWER / num_of_testcases),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(
                SUM_SCORING_METHOD.max_score,
            )),
        },
        id: 1u64,
    };

    let mut score = OneOfScore::default();
    evaluate_subtask_score(
        &subtask_result_correct.testcase_results,
        &SUM_SCORING_METHOD,
        &mut score,
    );
    assert_eq!(score, subtask_result_correct.score);
}

#[test]
fn evaluate_wrong_double_subtask_with_min_test() {
    let subtask_result_wrong = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(partial_score(3f64, 4f64)),
            get_double_testcase(DOUBLE_WRONG_ANSWER),
            get_double_testcase(partial_score(1f64, 4f64)),
            get_double_testcase(DOUBLE_CORRECT_ANSWER),
            get_double_testcase(DOUBLE_CORRECT_ANSWER),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(DOUBLE_WRONG_ANSWER)),
        },
        id: 1u64,
    };

    let mut score = OneOfScore::default();
    evaluate_subtask_score(
        &subtask_result_wrong.testcase_results,
        &MIN_SCORING_METHOD,
        &mut score,
    );
    assert_eq!(score, subtask_result_wrong.score);
}

#[test]
fn evaluate_partial_double_subtask_with_min_test() {
    let subtask_result_partial = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(partial_score(1f64, 5f64)),
            get_double_testcase(partial_score(2f64, 5f64)),
            get_double_testcase(partial_score(3f64, 5f64)),
            get_double_testcase(partial_score(4f64, 5f64)),
            get_double_testcase(DOUBLE_CORRECT_ANSWER),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(
                MIN_SCORING_METHOD.max_score * partial_score(1f64, 5f64),
            )),
        },
        id: 1u64,
    };

    let mut score = OneOfScore::default();
    evaluate_subtask_score(
        &subtask_result_partial.testcase_results,
        &MIN_SCORING_METHOD,
        &mut score,
    );
    assert_eq!(score, subtask_result_partial.score);
}

#[test]
fn evaluate_correct_double_subtask_with_min_test() {
    let subtask_result_correct = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(DOUBLE_CORRECT_ANSWER),
            get_double_testcase(DOUBLE_CORRECT_ANSWER),
            get_double_testcase(DOUBLE_CORRECT_ANSWER),
            get_double_testcase(DOUBLE_CORRECT_ANSWER),
            get_double_testcase(DOUBLE_CORRECT_ANSWER),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(
                MIN_SCORING_METHOD.max_score,
            )),
        },
        id: 1u64,
    };

    let mut score = OneOfScore::default();
    evaluate_subtask_score(
        &subtask_result_correct.testcase_results,
        &MIN_SCORING_METHOD,
        &mut score,
    );
    assert_eq!(score, subtask_result_correct.score);
}

#[test]
fn evaluate_submission_score_test() {
    let num_of_subtasks = 5f64;
    let max_score = 100f64;
    let subtask_result_wrong = SubtaskResult {
        testcase_results: vec![
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(false),
            get_bool_testcase(true),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(0f64)),
        },
        id: 1u64,
    };

    let subtask_result_correct = SubtaskResult {
        testcase_results: vec![
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(true),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(
                max_score / num_of_subtasks,
            )),
        },
        id: 1u64,
    };

    let num_of_correct_subtasks = 3f64;
    let submission_score = OneOfScore {
        score: Some(one_of_score::Score::DoubleScore(
            max_score * num_of_correct_subtasks / num_of_subtasks,
        )),
    };

    let mut score = OneOfScore::default();
    evaluate_submission_score(
        &[
            subtask_result_correct.clone(),
            subtask_result_correct.clone(),
            subtask_result_wrong.clone(),
            subtask_result_wrong,
            subtask_result_correct,
        ],
        &mut score,
    );
    assert_eq!(score, submission_score);
}
*/
