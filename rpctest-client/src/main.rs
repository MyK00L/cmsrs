use protos::service::test::{test_client::*, test_server::*, *};
use protos::ChannelTrait;

use std::{thread, time};
use tonic::Response;

#[derive(Debug, Default)]
pub struct MockTest {}

// Implement the service function(s) defined in the proto
// for the Greeter service (SayHello...)
#[tonic::async_trait]
impl Test for MockTest {
    async fn test_string(
        &self,
        request: tonic::Request<StringRequest>,
    ) -> Result<tonic::Response<StringResponse>, tonic::Status> {
        let addr = request.remote_addr();
        let inner = request.into_inner();
        eprintln!("received request with value {:?} from {:?}", inner, addr,);
        let reply = StringResponse {
            str: format!("Mock {}", inner.str),
        };
        Ok(Response::new(reply))
    }
}

async fn not_main<C: ChannelTrait>(
    mut client: TestClient<C>,
) -> Result<(), Box<dyn std::error::Error>> {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TestClient::connect("http://rpc-server:50051").await?;
    not_main(client).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::block_on;
    use tower::ServiceBuilder;
    #[test]
    // not an actual test, since not_main runs infinitely, but you get the point
    fn test_test() {
        let channel = ServiceBuilder::new().service(TestServer::new(MockTest::default()));
        let client = TestClient::new(channel);
        block_on(not_main(client)).unwrap();
    }
}
