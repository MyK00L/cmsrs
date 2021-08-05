use protos::service::test::{test_client::*, test_server::*, *};
use protos::ChannelTrait;
use tonic::transport::Channel;
use tonic::{transport::Server, Response};

#[cfg(test)]
mod tests;

#[derive(Debug, Default)]
pub struct MyTest<T: ChannelTrait> {
    test_client: Option<TestClient<T>>,
}

// Implement the service function(s) defined in the proto
// for the Greeter service (SayHello...)
#[tonic::async_trait]
impl<T: ChannelTrait + Send> Test for MyTest<T> {
    async fn test_string(
        &self,
        request: tonic::Request<StringRequest>,
    ) -> Result<tonic::Response<StringResponse>, tonic::Status> {
        let mut test_client = self.test_client.clone().unwrap();
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
    let addr = "0.0.0.0:50051".parse()?;
    let greeter = MyTest::<Channel> { test_client: None };

    println!("Starting gRPC Server...");
    Server::builder()
        .add_service(TestServer::new(greeter))
        .serve(addr)
        .await?;
    Ok(())
}
