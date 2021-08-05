use protos::service::test::{test_client::*, test_server::*, *};
use protos::ChannelTrait;
use protos::ADDR;
use tonic::{transport::*, Response};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct MyTest<T: ChannelTrait> {
    test_client: TestClient<T>,
}

impl<T: ChannelTrait> MyTest<T> {
    fn from_channels(channels: &[T]) -> Self {
        MyTest {
            test_client: TestClient::new(channels[0].clone()),
        }
    }
}
impl MyTest<Channel> {
    fn from_addresses(addresses: &'static [&str]) -> Self {
        Self::from_channels(
            &(addresses
                .iter()
                .map(|x| Channel::from_static(x).connect_lazy().unwrap())
                .collect::<Vec<Channel>>()),
        )
    }
}
impl Default for MyTest<Channel> {
    fn default() -> Self {
        Self::from_addresses(&[ADDR.test_client])
    }
}

// Implement the service function(s) defined in the proto
// for the Greeter service (SayHello...)
#[tonic::async_trait]
impl<T: ChannelTrait> Test for MyTest<T> {
    async fn test_string(
        &self,
        request: tonic::Request<StringRequest>,
    ) -> Result<tonic::Response<StringResponse>, tonic::Status> {
        let mut test_client = self.test_client.clone();
        let addr = request.remote_addr();
        let inner = request.into_inner();
        test_client
            .log_string(tonic::Request::new(LogRequest {
                str: format!("received request with value {:?} from {:?}", inner, addr),
            }))
            .await?;
        let reply = StringResponse {
            str: format!("Hello {}", inner.str),
        };
        Ok(Response::new(reply))
    }
    async fn log_string(
        &self,
        request: tonic::Request<LogRequest>,
    ) -> Result<tonic::Response<LogResponse>, tonic::Status> {
        let inner = request.into_inner();
        eprintln!("{:?}", inner);
        Ok(Response::new(LogResponse {}))
    }
}

// Use the tokio runtime to run our server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    //let addr = "[::1]:50051".parse()?;
    let addr = ADDR.test_server.parse()?;
    let greeter = MyTest::<Channel>::default();

    println!("Starting gRPC Server...");
    Server::builder()
        .add_service(TestServer::new(greeter))
        .serve(addr)
        .await?;
    Ok(())
}
