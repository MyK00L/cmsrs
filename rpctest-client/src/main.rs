use tonic::{transport::Server, Request, Response, Status};
use protos::worker_service::worker_service_client::*;
use protos::worker_service::worker_service_server::*;
use protos::worker_service::*;
use tonic;
use tokio;

use std::{thread, time};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = WorkerServiceClient::connect("http://rpc-server:50051").await?;
    let mut index = 1u32;

    loop {
        let request = tonic::Request::new(TestRequest{ name: "Request #".to_owned() + &index.to_string() });

        println!("Sending request to gRPC Server...");
        let response = client.test_worker(request).await?;

        println!("RESPONSE={:?}", response);

        index += 1;

        thread::sleep(time::Duration::from_secs(1));
    }

    Ok(())
}
