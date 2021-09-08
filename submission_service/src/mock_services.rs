use protos::{
    self,
    common::Resources,
    evaluation::{
        compilation_result, testcase_result, CompilationResult, EvaluationResult, SubtaskResult,
        TestcaseResult,
    },
    service::dispatcher::{EvaluateSubmissionResponse, MockDispatcher},
    *,
};
use protos::{
    scoring::{one_of_score, OneOfScore},
    service::evaluation::{problem, GetProblemResponse, MockEvaluation},
};
use rand::Rng;

fn generate_testcase_result() -> TestcaseResult {
    let mut gen = rand::thread_rng();
    let outcome = gen.gen::<i32>().checked_abs().unwrap_or(0) % 6;
    TestcaseResult {
        outcome,
        score: if outcome == testcase_result::Outcome::Ok as i32 {
            OneOfScore {
                score: Some(one_of_score::Score::BoolScore(true)),
            }
        } else {
            OneOfScore {
                score: Some(one_of_score::Score::BoolScore(false)),
            }
        },
        used_resources: Resources {
            time: common::Duration {
                secs: gen.gen(),
                nanos: gen.gen(),
            },
            memory_bytes: gen.gen(),
        },
    }
}

fn generate_subtask_result() -> SubtaskResult {
    SubtaskResult {
        testcase_results: vec![
            generate_testcase_result(),
            generate_testcase_result(),
            generate_testcase_result(),
            generate_testcase_result(),
            generate_testcase_result(),
        ],
        score: OneOfScore::default(),
    }
}

fn generate_min_subtask_scoring() -> protos::service::evaluation::Subtask {
    protos::service::evaluation::Subtask {
        id: 0u64,
        scoring: scoring::Subtask {
            method: scoring::subtask::Method::Min as i32,
            max_score: 20f64,
        },
        testcases_id: vec![], // now useless
    }
}

fn mock_dispatcher_init(mock_dispatcher: &mut MockDispatcher) {
    mock_dispatcher.evaluate_submission_set(EvaluateSubmissionResponse {
        res: EvaluationResult {
            compilation_result: CompilationResult {
                outcome: compilation_result::Outcome::Success as i32,
                used_resources: Resources {
                    time: common::Duration {
                        secs: 0u64,
                        nanos: 1u32,
                    },
                    memory_bytes: 1u64,
                },
                error_message: None,
            },
            subtask_results: vec![
                generate_subtask_result(),
                generate_subtask_result(),
                generate_subtask_result(),
                generate_subtask_result(),
                generate_subtask_result(),
                generate_subtask_result(),
            ],
            score: OneOfScore {
                score: Some(one_of_score::Score::DoubleScore(0f64)),
            },
        },
    });
}

fn mock_evaluation_init(mock_evaluation_server: &mut MockEvaluation, problem_id: u64) {
    mock_evaluation_server.get_problem_set(GetProblemResponse {
        info: protos::service::evaluation::Problem {
            id: problem_id,
            scoring: scoring::Problem {
                method: scoring::problem::Method::MaxSum as i32,
            },
            r#type: problem::Type::Other as i32,
            execution_limits: Resources {
                time: common::Duration {
                    secs: 0u64,
                    nanos: 1u32,
                },
                memory_bytes: 1u64,
            },
            compilation_limits: Resources {
                time: common::Duration {
                    secs: 0u64,
                    nanos: 1u32,
                },
                memory_bytes: 1u64,
            },
            subtasks: vec![
                generate_min_subtask_scoring(),
                generate_min_subtask_scoring(),
                generate_min_subtask_scoring(),
                generate_min_subtask_scoring(),
                generate_min_subtask_scoring(),
                generate_min_subtask_scoring(),
            ],
        },
    });
}

pub fn get_mock_dispatcher() -> MockDispatcher {
    let mut mock = MockDispatcher::default();
    mock_dispatcher_init(&mut mock);
    mock
}

pub fn get_mock_evaluation(problem_id: u64) -> MockEvaluation {
    let mut mock = MockEvaluation::default();
    mock_evaluation_init(&mut mock, problem_id);
    mock
}
