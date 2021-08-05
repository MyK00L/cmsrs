use protos::service::test::{test_client::*, *};
use protos::ChannelTrait;

use std::{thread, time};

async fn not_main<C: ChannelTrait>(
    mut client: TestClient<C>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut index = 1u32;
    loop {
        let request = tonic::Request::new(StringRequest {
            str: format!("Request#{}", index),
        });

        println!("Sending request to gRPC Server...");
        let response = client.test_string(request).await;

        println!("RESPONSE={:?}", response);

        index += 1;

        thread::sleep(time::Duration::from_millis(500));
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel =
        tonic::transport::Channel::from_static("http://rpc-server:50051").connect_lazy()?;
    let client = TestClient::new(channel);
    not_main(client).await?;
    Ok(())
}
