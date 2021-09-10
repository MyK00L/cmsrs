use core::panic;

use protos::evaluation::{SubtaskResult, TestcaseResult};
use protos::scoring::{self, one_of_score, OneOfScore, Subtask};

pub fn as_double(score: &OneOfScore) -> f64 {
    match score.score {
        Some(one_of_score::Score::DoubleScore(double_score)) => double_score,
        _ => panic!("Cannot cast OneOfScore to double"),
    }
}

pub fn as_bool(score: &OneOfScore) -> bool {
    match score.score {
        Some(one_of_score::Score::BoolScore(bool_score)) => bool_score,
        _ => panic!("Cannot cast OneOfScore to bool"),
    }
}

pub fn score_with_bool(bool_score: bool) -> OneOfScore {
    OneOfScore {
        score: Some(one_of_score::Score::BoolScore(bool_score)),
    }
}

pub fn score_with_double(double_score: f64) -> OneOfScore {
    OneOfScore {
        score: Some(one_of_score::Score::DoubleScore(double_score)),
    }
}

fn boolean_testcases(testcases: &[TestcaseResult]) -> bool {
    testcases.iter().all(|testcase_result| {
        matches!(
            testcase_result.score.score,
            Some(one_of_score::Score::BoolScore(_))
        )
    })
}

fn double_testcases(testcases: &[TestcaseResult]) -> bool {
    testcases.iter().all(|testcase_result| {
        matches!(
            testcase_result.score.score,
            Some(one_of_score::Score::DoubleScore(_))
        )
    })
}

fn evaluate_bool_subtask_with_min(testcases: &[TestcaseResult], max_score: f64) -> OneOfScore {
    score_with_double(
        match testcases
            .iter()
            .map(|t| as_bool(&t.score))
            .fold(true, |a, b| a & b)
        {
            true => max_score,
            false => 0f64,
        },
    )
}

fn evaluate_bool_subtask_with_sum(testcases: &[TestcaseResult], max_score: f64) -> OneOfScore {
    let correct = testcases
        .iter()
        .map(|t| as_bool(&t.score))
        .filter(|score| *score)
        .count() as f64;

    score_with_double(correct * max_score / (testcases.len() as f64))
}

fn evaluate_double_subtask_with_min(testcases: &[TestcaseResult], max_score: f64) -> OneOfScore {
    score_with_double(
        max_score
            * testcases
                .iter()
                .map(|t| as_double(&t.score))
                .fold(1f64, f64::min),
    )
}

fn evaluate_double_subtask_with_sum(testcases: &[TestcaseResult], max_score: f64) -> OneOfScore {
    score_with_double(
        max_score
            * testcases
                .iter()
                .map(|t| as_double(&t.score))
                .fold(0f64, |a, b| a + b),
    )
}

pub fn evaluate_subtask_score(
    testcases: &[TestcaseResult],
    scoring_method: &Subtask,
    result: &mut OneOfScore,
) {
    *result = if boolean_testcases(testcases) {
        if scoring_method.method == scoring::subtask::Method::Min as i32 {
            evaluate_bool_subtask_with_min(testcases, scoring_method.max_score)
        } else {
            // scoring_method.method == scoring::subtask::Method::Sum as i32
            evaluate_bool_subtask_with_sum(testcases, scoring_method.max_score)
        }
    } else if double_testcases(testcases) {
        if scoring_method.method == scoring::subtask::Method::Min as i32 {
            evaluate_double_subtask_with_min(testcases, scoring_method.max_score)
        } else {
            // scoring_method.method == scoring::subtask::Method::Sum as i32
            evaluate_double_subtask_with_sum(testcases, scoring_method.max_score)
        }
    } else {
        panic!("The type of the testcases scores is not consinstent")
    };
}

/// Pre:
///      - the score of every single subtask has already been calculated
///      - subtasks' scores are all double
pub fn evaluate_submission_score(subtasks: &[SubtaskResult], result: &mut OneOfScore) {
    *result = score_with_double(
        subtasks
            .iter()
            .map(|t| as_double(&t.score))
            .fold(0f64, |a, b| a + b),
    );
}
