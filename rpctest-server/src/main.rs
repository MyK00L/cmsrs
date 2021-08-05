use protos::service::test::{test_server::*, *};
use protos::utils::*;
use tonic::{transport::*, Response};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct MyTest<T: ChannelTrait> {
    client_manager: ClientManager<T>,
}

impl Default for MyTest<Channel> {
    fn default() -> Self {
        Self {
            client_manager: ClientManager::default(),
        }
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
        let mut test_client = self.client_manager.test_client.clone();
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
    let addr = get_local_address(Service::TEST).parse()?;
    let greeter = MyTest::<Channel>::default();

    println!("Starting gRPC Server...");
    Server::builder()
        .add_service(TestServer::new(greeter))
        .serve(addr)
        .await?;
    Ok(())
}
