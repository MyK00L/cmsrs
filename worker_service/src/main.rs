use protos::{common::{self, Resources}, evaluation::{CompilationResult, TestcaseResult, compilation_result, testcase_result::Outcome}, scoring::{OneOfScore, one_of_score}, service::worker::{
        worker_server::{Worker, WorkerServer},
        EvaluateSubmissionRequest, EvaluateSubmissionResponse, UpdateSourceRequest,
        UpdateSourceResponse, UpdateTestcaseRequest, UpdateTestcaseResponse,
    }, utils::{get_local_address, Service}};
use tonic::{transport::Server, Request, Response, Status};
use rand::seq::SliceRandom;
use rand::thread_rng;
use utils::scoring_lib::score_with_bool;

pub struct WorkerService {}

impl WorkerService {
    fn new() -> Self {
        WorkerService {}
    }
}

const NUM_OF_SUBTASKS: u64 = 5;
const NUM_OF_TESTCASES_PER_SUBTASK: u64 = 5;

#[tonic::async_trait]
impl Worker for WorkerService {
    async fn evaluate_submission(
        &self,
        request: Request<EvaluateSubmissionRequest>,
    ) -> Result<Response<EvaluateSubmissionResponse>, Status> {
        Ok(Response::new(EvaluateSubmissionResponse {
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
            testcase_results: {
                let mut testcase_results =
                    Vec::with_capacity((NUM_OF_TESTCASES_PER_SUBTASK * NUM_OF_SUBTASKS) as usize);
                for i in 0..NUM_OF_TESTCASES_PER_SUBTASK * NUM_OF_SUBTASKS {
                    testcase_results.push(TestcaseResult {
                        outcome: Outcome::Ok as i32,
                        score: score_with_bool(true),
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
        }))
    }

    async fn update_testcase(
        &self,
        request: Request<UpdateTestcaseRequest>,
    ) -> Result<Response<UpdateTestcaseResponse>, Status> {
        todo!()
    }

    async fn update_source(
        &self,
        request: Request<UpdateSourceRequest>,
    ) -> Result<Response<UpdateSourceResponse>, Status> {
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr: _ = get_local_address(Service::WORKER).parse()?;
    let worker_service = WorkerService::new();

    println!("Starting a worker server");
    Server::builder()
        .add_service(WorkerServer::new(worker_service))
        .serve(addr)
        .await?;
    Ok(())
}
