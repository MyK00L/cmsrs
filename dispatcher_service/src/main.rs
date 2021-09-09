use protos::{
    evaluation::{compilation_result, EvaluationResult, SubtaskResult, TestcaseResult},
    scoring::OneOfScore,
    service::{
        dispatcher::{
            self,
            dispatcher_server::{Dispatcher, DispatcherServer},
        },
        evaluation::{evaluation_server::Evaluation, GetProblemRequest, MockEvaluation},
        worker::{self, worker_server::Worker, MockWorker},
    },
    utils::{get_local_address, Service},
};
use tonic::{transport::Server, Request, Response, Status};

struct RoundRobin {
    actual: usize,
    total_number: usize,
}

static mut SELECTOR: RoundRobin = RoundRobin {
    actual: 0,
    total_number: 0,
};

impl RoundRobin {
    fn next(&mut self) -> usize {
        let next_worker_id = self.actual;
        self.actual = (self.actual + 1) % self.total_number;
        next_worker_id
    }
}

pub struct DispatcherService {
    // worker list
    workers: Vec<MockWorker>,
    // Balance::new(tower::discover::ServiceList::new(vec![svc1, svc2]))
}

impl DispatcherService {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // somehow get the worker list
        // or get the reference for a worker provider
        let workers = vec![MockWorker::default(), MockWorker::default()];
        let len = workers.len();
        unsafe {
            SELECTOR.total_number = len;
        }
        Ok(Self { workers })
    }
}

fn dispatcher_to_worker_request(
    dispatcher_request: &dispatcher::EvaluateSubmissionRequest,
) -> worker::EvaluateSubmissionRequest {
    worker::EvaluateSubmissionRequest {
        problem_id: dispatcher_request.sub.problem_id,
        source: dispatcher_request.sub.source.clone(),
    }
}

// Assuming that testcase_results are in the correct order. The only 
// problem metadata we need are the number of testcases in each subtask
async fn group_testcases(mut testcase_results: Vec<TestcaseResult>, problem_id: u64) -> Result<Vec<SubtaskResult>, Status> {
    let mut mock_evaluation_service = MockEvaluation::default();
    // mock_evaluation_service.get_problem_set(...stuff...)
    let problem_metadata = mock_evaluation_service
        .get_problem(Request::new(GetProblemRequest { problem_id }))
        .await?
        .into_inner();

    testcase_results.reverse(); // TODO change this hacky code
    let mut remaining_len = testcase_results.len();

    Ok(problem_metadata.info.subtasks
        .iter()
        .map(|subtask| {
            let chunk_size = subtask.testcases_id.len();
            SubtaskResult {
                testcase_results: {
                    let chunk = testcase_results.split_off(remaining_len - chunk_size); // TODO change this hacky code
                    remaining_len -= chunk_size;
                    chunk.to_vec()
                },
                score: OneOfScore::default(),
            }
        })
        .collect())
}

async fn worker_to_dispatcher_response(
    worker_response: &worker::EvaluateSubmissionResponse,
    problem_id: u64,
) -> Result<dispatcher::EvaluateSubmissionResponse, Status> {
    // in any case:
    //     - res.compilation_result = worker_response.compilation_result
    //     - res.score: not relevant. Use OneOfScore::default()
    // if compilation is successfull:
    //     - res.subtask_results: group worker_response.testcase_results based on the
    //       metadata of the Problem obtained by the RPC invocation get_problem (of
    //       the evaluation_service)
    // if compilation unsuccessfull:
    //     - res.subtask_results == vec![]
    Ok(dispatcher::EvaluateSubmissionResponse {
        res: EvaluationResult {
            compilation_result: worker_response.compilation_result.clone(),
            subtask_results: if worker_response.compilation_result.outcome
                == compilation_result::Outcome::Success as i32
            {
                group_testcases(worker_response.testcase_results.clone(), problem_id)
                    .await?
            } else {
                vec![]
            },
            score: OneOfScore::default(),
        },
    })
}

#[tonic::async_trait]
impl Dispatcher for DispatcherService {
    async fn evaluate_submission(
        &self,
        request: Request<dispatcher::EvaluateSubmissionRequest>,
    ) -> Result<Response<dispatcher::EvaluateSubmissionResponse>, Status> {
        let submission_request = request.into_inner();
        let problem_id = submission_request.sub.problem_id;
        let worker_request = Request::new(dispatcher_to_worker_request(&submission_request));

        let worker_response = self.workers[unsafe { SELECTOR.next() }] // gets the worker we want to use
            .evaluate_submission(worker_request)
            .await?
            .into_inner();

        worker_to_dispatcher_response(&worker_response, problem_id).await.map(Response::new)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = get_local_address(Service::DISPATCHER).parse()?;
    let dispatcher_service = DispatcherService::new().await?;

    println!("Starting dispatcher server");
    Server::builder()
        .add_service(DispatcherServer::new(dispatcher_service))
        .serve(addr)
        .await?;
    Ok(())
}
