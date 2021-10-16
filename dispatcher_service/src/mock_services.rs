use protos::{
    common::{self, Resources, Score},
    evaluation::{compilation_result, testcase_result::Outcome, CompilationResult, TestcaseResult},
    scoring::{self},
    service::{
        evaluation::{problem, GetProblemResponse, MockEvaluation},
        worker::{self, MockWorker},
    },
};
use rand::seq::SliceRandom;
use rand::thread_rng;

const NUM_OF_SUBTASKS: u64 = 5;
const NUM_OF_TESTCASES_PER_SUBTASK: u64 = 5;

fn generate_min_subtask_scoring(subtask_num: u64) -> protos::service::evaluation::Subtask {
    protos::service::evaluation::Subtask {
        id: 0u64,
        scoring: scoring::Subtask {
            method: scoring::subtask::Method::Min as i32,
            max_score: Score { score: 20f64 },
        },
        testcases_id: {
            let mut testcase_ids = Vec::with_capacity(NUM_OF_TESTCASES_PER_SUBTASK as usize);
            for i in 0..NUM_OF_TESTCASES_PER_SUBTASK {
                testcase_ids.push(subtask_num * NUM_OF_TESTCASES_PER_SUBTASK + i);
            }
            testcase_ids
        }, // the only relevant field
    }
}

fn mock_evaluation_init(mock_evaluation_server: &mut MockEvaluation) {
    mock_evaluation_server.get_problem_set(GetProblemResponse {
        info: protos::service::evaluation::Problem {
            id: 0u64,
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
            subtasks: {
                let mut subtasks = Vec::with_capacity(NUM_OF_SUBTASKS as usize);
                for i in 0..NUM_OF_SUBTASKS {
                    subtasks.push(generate_min_subtask_scoring(i));
                }
                subtasks
            },
        },
    });
}

#[allow(dead_code)]
fn mock_worker_init(mock_worker_server: &mut MockWorker) {
    mock_worker_server.evaluate_submission_set(worker::EvaluateSubmissionResponse {
        compilation_result: CompilationResult {
            outcome: compilation_result::Outcome::Success as i32,
            used_resources: Resources {
                time: common::Duration {
                    secs: 0u64,
                    nanos: 1u32,
                },
                memory_bytes: 1u64,
            },
        },
        testcase_results: {
            let mut testcase_results =
                Vec::with_capacity((NUM_OF_TESTCASES_PER_SUBTASK * NUM_OF_SUBTASKS) as usize);
            for i in 0..NUM_OF_TESTCASES_PER_SUBTASK * NUM_OF_SUBTASKS {
                testcase_results.push(TestcaseResult {
                    outcome: Outcome::Ok as i32,
                    score: Score { score: 1f64 },
                    used_resources: Resources {
                        time: common::Duration {
                            secs: 0u64,
                            nanos: 1u32,
                        },
                        memory_bytes: 1u64,
                    },
                    id: i,
                });
            }
            testcase_results.shuffle(&mut thread_rng());
            testcase_results
        },
    })
}

pub fn get_mock_evaluation() -> MockEvaluation {
    let mut mock = MockEvaluation::default();
    mock_evaluation_init(&mut mock);
    mock
}

#[allow(dead_code)]
pub fn get_mock_worker() -> MockWorker {
    let mut mock = MockWorker::default();
    mock_worker_init(&mut mock);
    mock
}
