use protos::service::test::{test_client::*, test_server::*, *};
use protos::utils::*;
use tonic::{transport::*, Response};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct MyTest<TC: ChannelTrait> {
    test_client: TestClient<TC>,
}

impl Default for MyTest<Channel> {
    fn default() -> Self {
        Self {
            test_client: TestClient::new(get_new_channel(Service::TEST)),
        }
    }
}

// Implement the service function(s) defined in the proto
// for the Greeter service (SayHello...)
#[tonic::async_trait]
impl<T: ChannelTrait> Test for MyTest<T> {
    async fn test_string(
        &self,
        request: tonic::Request<TestStringRequest>,
    ) -> Result<tonic::Response<TestStringResponse>, tonic::Status> {
        let mut test_client = self.test_client.clone();
        let addr = request.remote_addr();
        let inner = request.into_inner();
        test_client
            .log_string(tonic::Request::new(LogStringRequest {
                str: format!("received request with value {:?} from {:?}", inner, addr),
            }))
            .await?;
        eprintln!(
            "mock test: {:?}",
            test_client
                .test_string(tonic::Request::new(TestStringRequest {
                    str: String::from("(ᓀ ᓀ)")
                }))
                .await?
        );
        let reply = TestStringResponse {
            str: format!("Hello {}", inner.str),
        };
        Ok(Response::new(reply))
    }
    async fn log_string(
        &self,
        request: tonic::Request<LogStringRequest>,
    ) -> Result<tonic::Response<LogStringResponse>, tonic::Status> {
        let inner = request.into_inner();
        eprintln!("{:?}", inner);
        Ok(Response::new(LogStringResponse {}))
    }
    async fn file(&self, req: tonic::Request<tonic::Streaming<FileRequest>>) -> Result<tonic::Response<<Self as protos::service::test::test_server::Test>::fileStream>, tonic::Status> { todo!() }
    type fileStream = tonic::Streaming<FileResponse>;
}

// Use the tokio runtime to run our server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = get_local_address(Service::TEST).parse()?;
    let greeter = MyTest::<Channel>::default();

    println!("Starting gRPC Server...");
    Server::builder()
        .add_service(TestServer::new(greeter))
        .serve(addr)
        .await?;
    Ok(())
}
