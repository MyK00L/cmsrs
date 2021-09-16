use protos::{
    service::worker::{
        worker_server::{Worker, WorkerServer},
        EvaluateSubmissionRequest, EvaluateSubmissionResponse, UpdateSourceRequest,
        UpdateSourceResponse, UpdateTestcaseRequest, UpdateTestcaseResponse,
    },
    utils::{get_local_address, Service},
};
use tonic::{transport::Server, Request, Response, Status};

pub struct WorkerService {}

impl WorkerService {
    fn new() -> Self {
        WorkerService {}
    }
}

#[tonic::async_trait]
impl Worker for WorkerService {
    async fn evaluate_submission(
        &self,
        _request: Request<EvaluateSubmissionRequest>,
    ) -> Result<Response<EvaluateSubmissionResponse>, Status> {
        todo!()
    }

    async fn update_testcase(
        &self,
        _request: Request<UpdateTestcaseRequest>,
    ) -> Result<Response<UpdateTestcaseResponse>, Status> {
        todo!()
    }

    async fn update_source(
        &self,
        _request: Request<UpdateSourceRequest>,
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
