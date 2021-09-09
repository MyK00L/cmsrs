use protos::{service::dispatcher::{self, dispatcher_server::{Dispatcher, DispatcherServer}}, utils::{Service, get_local_address}};
use tonic::{Request, Response, Status, transport::Server};


pub struct DispatcherService {
    // worker list
    test_field: i32
}

impl DispatcherService {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // somehow get the worker list
        // or get the reference for a worker provider
        Ok( Self { test_field: 0i32 } )
    }
}

#[tonic::async_trait]
impl Dispatcher for DispatcherService {
    async fn evaluate_submission(
        &self,
        request: Request<dispatcher::EvaluateSubmissionRequest>,
    ) -> Result<Response<dispatcher::EvaluateSubmissionResponse>,Status> {
        todo!()
    }
}

#[tokio::main]
async fn main () -> Result<(), Box<dyn std::error::Error>>{
    let addr = get_local_address(Service::DISPATCHER).parse()?;
    let dispatcher_service = DispatcherService::new().await?;

    println!("Starting dispatcher server");
    Server::builder()
        .add_service(DispatcherServer::new(dispatcher_service))
        .serve(addr)
        .await?;
    Ok(())
}