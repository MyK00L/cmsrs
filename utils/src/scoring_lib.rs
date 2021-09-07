use core::panic;

use protos::evaluation::{SubtaskResult, TestcaseResult};
use protos::scoring::{one_of_score, OneOfScore, Subtask};

fn as_double(score: &OneOfScore) -> f64 {
    match score.score {
        Some(one_of_score::Score::DoubleScore(double_score)) => double_score,
        _ => panic!("Cannot cast OneOfScore to double"),
    }
}

fn as_bool(score: &OneOfScore) -> bool {
    match score.score {
        Some(one_of_score::Score::BoolScore(bool_score)) => bool_score,
        _ => panic!("Cannot cast OneOfScore to bool"),
    }
}

fn score_with_double(double_score: f64) -> OneOfScore {
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

pub fn evaluate_subtask_score(
    testcases: Vec<TestcaseResult>,
    scoring_method: &Subtask,
) -> OneOfScore {
    let testcases_acc = if boolean_testcases(&testcases) {
        // this subtask has boolean scores
        match scoring_method.method {
            0 => {
                // Min
                match testcases
                    .iter()
                    .map(|t| as_bool(&t.score))
                    .fold(true, |a, b| a & b)
                {
                    true => 1f64,
                    false => 0f64,
                }
            }
            1 => {
                // Sum
                let correct = testcases
                    .iter()
                    .map(|t| as_bool(&t.score))
                    .filter(|score| *score)
                    .count() as f64;

                correct / (testcases.len() as f64)
            }
            _ => panic!(),
        }
    } else if double_testcases(&testcases) {
        // this subtask has double scores
        let init = match scoring_method.method {
            0 => 1f64, // Min
            1 => 0f64, // Sum
            _ => panic!(),
        };

        testcases
            .iter()
            .map(|t| as_double(&t.score))
            .fold(init, |a, b| {
                match scoring_method.method {
                    0 => f64::min(a, b), // Min
                    _ => a + b,          // Sum
                }
            })
    } else {
        panic!("The type of the testcases scores is not consinstent")
    };

    score_with_double(testcases_acc * scoring_method.max_score)
}

/// Pre:
///      - the score of every single subtask has already been calculated
///      - subtasks' scores are all double
pub fn evaluate_submission_score(subtasks: Vec<SubtaskResult>) -> OneOfScore {
    score_with_double(
        subtasks
            .iter()
            .map(|t| as_double(&t.score))
            .fold(0f64, |a, b| a + b),
    )
}
