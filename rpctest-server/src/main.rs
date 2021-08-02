use protos::service::test::{test_server::*, *};
use tonic::{transport::Server, Response};

#[derive(Debug, Default)]
pub struct MyTest {}

// Implement the service function(s) defined in the proto
// for the Greeter service (SayHello...)
#[tonic::async_trait]
impl Test for MyTest {
    async fn test_string(
        &self,
        request: tonic::Request<StringRequest>,
    ) -> Result<tonic::Response<StringResponse>, tonic::Status> {
        let addr = request.remote_addr();
        let inner = request.into_inner();
        eprintln!("received request with value {:?} from {:?}", inner, addr,);
        let reply = StringResponse {
            str: format!("Hello {}", inner.str),
        };
        Ok(Response::new(reply))
    }
}

// Use the tokio runtime to run our server
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "0.0.0.0:50051".parse()?;
    let greeter = MyTest::default();

    println!("Starting gRPC Server...");
    Server::builder()
        .add_service(TestServer::new(greeter))
        .serve(addr)
        .await?;
    Ok(())
}
