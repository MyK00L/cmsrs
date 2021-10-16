use protos::{
    common::Score,
    evaluation::{compilation_result, EvaluationResult, SubtaskResult, TestcaseResult},
    service::{
        dispatcher::{
            self,
            dispatcher_server::{Dispatcher, DispatcherServer},
        },
        evaluation::{evaluation_server::Evaluation, GetProblemRequest},
        worker::{self, worker_client::WorkerClient},
    },
    utils::{get_local_address, Service},
};
use std::collections::HashMap;
use tonic::transport::Channel;
use tonic::{transport::Server, Request, Response, Status};

mod mock_services;

pub struct DispatcherService {
    load_balancer: WorkerClient<Channel>,
}

impl DispatcherService {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // the format of the IP address in the docker network has form:
        // IMAGE_NAME://SERVICE_NAME:EXPOSED_PORT
        // NB: if you implemented the service (basically if you have written a Dockerfile for it),
        //     then the IMAGE_NAME and EXPOSED_PORT are specified in the Dockerfile of that service.
        //     If the service uses an image downloaded from the web (e.g. the database images like
        //     postgres or mongodb), then you should refer to the "docker-compose.yaml" file:
        //     IMAGE_NAME is the name of the image from the web
        //     EXPOSED_PORT is the last number of the string in the "ports" field
        let workers_ip = [
            "runtime://worker_service_1:50051",
            "runtime://worker_service_2:50051",
        ];

        let endpoints = workers_ip.iter().map(|ip| Channel::from_static(ip));

        let channel = Channel::balance_list(endpoints);

        Ok(Self {
            load_balancer: WorkerClient::new(channel),
        })
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

async fn group_testcases(
    testcase_results: Vec<TestcaseResult>,
    problem_id: u64,
) -> Result<Vec<SubtaskResult>, Status> {
    let mock_evaluation_service = mock_services::get_mock_evaluation();
    let problem_metadata = mock_evaluation_service
        .get_problem(Request::new(GetProblemRequest { problem_id }))
        .await?
        .into_inner();

    let map_id_to_testcase_result: HashMap<_, _> = testcase_results
        .iter()
        .map(|testcase_result| (testcase_result.id, testcase_result))
        .collect();

    Ok(problem_metadata
        .info
        .subtasks
        .iter()
        .map(|subtask| SubtaskResult {
            testcase_results: subtask
                .testcases_id
                .iter()
                .map(|testcase_id| map_id_to_testcase_result[testcase_id].to_owned())
                .collect(),
            score: Score { score: 0f64 },
            id: subtask.id,
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
                group_testcases(worker_response.testcase_results.clone(), problem_id).await?
            } else {
                vec![]
            },
            score: Score { score: 0f64 },
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

        let worker_response = self
            .load_balancer
            .clone()
            .evaluate_submission(worker_request)
            .await?
            .into_inner();

        worker_to_dispatcher_response(&worker_response, problem_id)
            .await
            .map(Response::new)
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
