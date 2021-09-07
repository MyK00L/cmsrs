use crate::scoring_lib::{evaluate_submission_score, evaluate_subtask_score};

use super::*;

use protos::common::{Duration, Resources};
use protos::evaluation::*;
use protos::evaluation::{SubtaskResult, TestcaseResult};
use protos::scoring::{one_of_score, subtask, OneOfScore, Subtask};

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

fn get_bool_testcase(result: bool) -> TestcaseResult {
    TestcaseResult {
        outcome: testcase_result::Outcome::Ok as i32,
        score: OneOfScore {
            score: Some(one_of_score::Score::BoolScore(result)),
        },
        used_resources: Resources {
            time: Duration { secs: 0, nanos: 0 },
            memory_bytes: 0u64,
        },
    }
}

/// Pre:
///    - result is in range \[0, 1\]
fn get_double_testcase(result: f64) -> TestcaseResult {
    TestcaseResult {
        outcome: testcase_result::Outcome::Ok as i32,
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(result)),
        },
        used_resources: Resources {
            time: Duration { secs: 0, nanos: 0 },
            memory_bytes: 0u64,
        },
    }
}

#[test]
fn evaluate_bool_subtask_with_min_test() {
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
            score: Some(one_of_score::Score::DoubleScore(max_score)),
        },
    };

    let scoring_method = Subtask {
        method: subtask::Method::Min as i32,
        max_score: max_score,
    };

    assert_eq!(
        evaluate_subtask_score(subtask_result_wrong.testcase_results, &scoring_method),
        subtask_result_wrong.score
    );

    assert_eq!(
        evaluate_subtask_score(subtask_result_correct.testcase_results, &scoring_method),
        subtask_result_correct.score
    );
}

#[test]
fn evaluate_bool_subtask_with_sum_test() {
    let max_score = 100f64;
    let subtask_result_wrong = SubtaskResult {
        testcase_results: vec![
            get_bool_testcase(false),
            get_bool_testcase(false),
            get_bool_testcase(false),
            get_bool_testcase(false),
            get_bool_testcase(false),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(0f64)),
        },
    };

    let subtask_result_partial = SubtaskResult {
        testcase_results: vec![
            get_bool_testcase(false),
            get_bool_testcase(true),
            get_bool_testcase(true),
            get_bool_testcase(false),
            get_bool_testcase(true),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(max_score * 3f64 / 5f64)),
        },
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
            score: Some(one_of_score::Score::DoubleScore(max_score)),
        },
    };

    let scoring_method = Subtask {
        method: subtask::Method::Sum as i32,
        max_score: max_score,
    };

    assert_eq!(
        evaluate_subtask_score(subtask_result_wrong.testcase_results, &scoring_method),
        subtask_result_wrong.score
    );

    assert_eq!(
        evaluate_subtask_score(subtask_result_partial.testcase_results, &scoring_method),
        subtask_result_partial.score
    );

    assert_eq!(
        evaluate_subtask_score(subtask_result_correct.testcase_results, &scoring_method),
        subtask_result_correct.score
    );
}

#[test]
fn evaluate_double_subtask_with_sum_test() {
    let num_of_tests = 5f64;
    let max_score = 100f64;
    let partial_score = |correct: f64, total: f64| correct / total;
    let wrong_answer = 0f64;
    let correct_answer = 1f64;
    let subtask_result_wrong = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(wrong_answer),
            get_double_testcase(wrong_answer),
            get_double_testcase(wrong_answer),
            get_double_testcase(wrong_answer),
            get_double_testcase(wrong_answer),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(wrong_answer)),
        },
    };

    let subtask_result_partial = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(partial_score(2f64, 5f64) / num_of_tests),
            get_double_testcase(partial_score(3f64, 5f64) / num_of_tests),
            get_double_testcase(wrong_answer),
            get_double_testcase(correct_answer / num_of_tests),
            get_double_testcase(partial_score(1f64, 5f64) / num_of_tests),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(
                max_score
                    * ((correct_answer + partial_score(2f64 + 3f64 + 1f64, 5f64)) / num_of_tests),
            )),
        },
    };

    let subtask_result_correct = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(correct_answer / num_of_tests),
            get_double_testcase(correct_answer / num_of_tests),
            get_double_testcase(correct_answer / num_of_tests),
            get_double_testcase(correct_answer / num_of_tests),
            get_double_testcase(correct_answer / num_of_tests),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(max_score)),
        },
    };

    let scoring_method = Subtask {
        method: subtask::Method::Sum as i32,
        max_score: max_score,
    };

    assert_eq!(
        evaluate_subtask_score(subtask_result_wrong.testcase_results, &scoring_method),
        subtask_result_wrong.score
    );

    let got = evaluate_subtask_score(subtask_result_partial.testcase_results, &scoring_method);
    let expected = subtask_result_partial.score;

    println!("got: {0:?}, expected: {1:?}", got, expected);
    // FLOATING POINT ERROR
    assert_eq!(got, expected);

    assert_eq!(
        evaluate_subtask_score(subtask_result_correct.testcase_results, &scoring_method),
        subtask_result_correct.score
    );
}

#[test]
fn evaluate_double_subtask_with_min_test() {
    let max_score = 100f64;
    let partial_score = |correct: f64, total: f64| correct / total;
    let wrong_answer = 0f64;
    let correct_answer = 1f64;
    let subtask_result_wrong = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(partial_score(3f64, 4f64)),
            get_double_testcase(wrong_answer),
            get_double_testcase(partial_score(1f64, 4f64)),
            get_double_testcase(correct_answer),
            get_double_testcase(correct_answer),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(wrong_answer)),
        },
    };

    let subtask_result_partial = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(partial_score(1f64, 5f64)),
            get_double_testcase(partial_score(2f64, 5f64)),
            get_double_testcase(partial_score(3f64, 5f64)),
            get_double_testcase(partial_score(4f64, 5f64)),
            get_double_testcase(correct_answer),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(
                max_score * partial_score(1f64, 5f64),
            )),
        },
    };

    let subtask_result_correct = SubtaskResult {
        testcase_results: vec![
            get_double_testcase(correct_answer),
            get_double_testcase(correct_answer),
            get_double_testcase(correct_answer),
            get_double_testcase(correct_answer),
            get_double_testcase(correct_answer),
        ],
        score: OneOfScore {
            score: Some(one_of_score::Score::DoubleScore(max_score)),
        },
    };

    let scoring_method = Subtask {
        method: subtask::Method::Min as i32,
        max_score: max_score,
    };

    assert_eq!(
        evaluate_subtask_score(subtask_result_wrong.testcase_results, &scoring_method),
        subtask_result_wrong.score
    );

    assert_eq!(
        evaluate_subtask_score(subtask_result_partial.testcase_results, &scoring_method),
        subtask_result_partial.score
    );

    assert_eq!(
        evaluate_subtask_score(subtask_result_correct.testcase_results, &scoring_method),
        subtask_result_correct.score
    );
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
    };

    let num_of_correct_subtasks = 3f64;
    let submission_score = OneOfScore {
        score: Some(one_of_score::Score::DoubleScore(
            max_score * num_of_correct_subtasks / num_of_subtasks,
        )),
    };

    assert_eq!(
        evaluate_submission_score(vec![
            subtask_result_correct.clone(),
            subtask_result_correct.clone(),
            subtask_result_wrong.clone(),
            subtask_result_wrong.clone(),
            subtask_result_correct
        ]),
        submission_score
    );
}
