use protos::service::test::{test_client::*, *};

use std::{thread, time};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = TestClient::connect("http://rpc-server:50051").await?;
    let mut index = 1u32;

    loop {
        let request = tonic::Request::new(StringRequest {
            str: format!("Request#{}", index),
        });

        println!("Sending request to gRPC Server...");
        let response = client.test_string(request).await?;

        println!("RESPONSE={:?}", response);

        index += 1;

        thread::sleep(time::Duration::from_millis(500));
    }
}
