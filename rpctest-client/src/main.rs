use tonic::{transport::Server, Request, Response, Status};
use protos::worker_service::worker_service_client::*;
use protos::worker_service::worker_service_server::*;
use protos::worker_service::*;
use tonic;
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = WorkerServiceClient::connect("http://[::1]:50051").await?;

    let request = tonic::Request::new(WorkerRequest::default());

    println!("Sending request to gRPC Server...");
    let response = client.evaluate_submission(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
}
