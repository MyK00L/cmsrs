use tonic::{transport::Server, Request, Response, Status};
use protos::worker_service::worker_service_client::*;
use protos::worker_service::worker_service_server::*;
use protos::worker_service::*;
use tonic;
use tokio;

#[derive(Debug, Default)]
pub struct MyWorker {}

// Implement the service function(s) defined in the proto
// for the Greeter service (SayHello...)
#[tonic::async_trait]
impl WorkerService for MyWorker {
    async fn test_worker(
        &self,
        request: tonic::Request<TestRequest>,
    ) -> Result<tonic::Response<TestResponse>, tonic::Status> {
        println!("Got a request: {:?}", request);

        let reply = TestResponse {
            message: format!("Hello {}!", request.into_inner().name).into(), // We must use .into_inner() as the fields of gRPC requests and responses are private
        };

        Ok(Response::new(reply))
    }
    async fn evaluate_submission(
        &self,
        request: tonic::Request<WorkerRequest>,
    ) -> Result<tonic::Response<WorkerResponse>, tonic::Status> {
        Ok(Response::new(WorkerResponse::default()))
    }
    async fn update_testcase(
        &self,
        request: tonic::Request<WorkerUpdateTestcaseRequest>,
    ) -> Result<tonic::Response<WorkerUpdateTestcaseResponse>, tonic::Status> {
        Ok(Response::new(WorkerUpdateTestcaseResponse::default()))
    }
    async fn update_source(
        &self,
        request: tonic::Request<WorkerUpdateSourceRequest>,
    ) -> Result<tonic::Response<WorkerUpdateSourceResponse>, tonic::Status> {
        Ok(Response::new(WorkerUpdateSourceResponse::default()))
    }
}

// Use the tokio runtime to run our server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    let greeter = MyWorker::default();

    println!("Starting gRPC Server...");
    Server::builder()
        .add_service(WorkerServiceServer::new(greeter))
        .serve(addr)
        .await?;

    Ok(())
}
