use super::*;
use tokio_test::block_on;
use tower::ServiceBuilder;

#[derive(Debug, Default, Clone)]
pub struct MockTest {}
#[tonic::async_trait]
impl Test for MockTest {
    async fn test_string(
        &self,
        request: tonic::Request<StringRequest>,
    ) -> Result<tonic::Response<StringResponse>, tonic::Status> {
        eprintln!("Mock test_string");
        Ok(Response::new(StringResponse {
            str: format!("Mock {}", inner.str),
        }))
    }
    async fn log_string(
        &self,
        request: tonic::Request<LogRequest>,
    ) -> Result<tonic::Response<LogResponse>, tonic::Status> {
        let inner = request.into_inner();
        eprintln!("Mock log_string received {:?}", inner);
        Ok(Response::new(LogResponse {}))
    }
}

#[test]
fn nope() {
    let channel = ServiceBuilder::new().service(TestServer::new(MockTest::default()));
    let client = TestClient::new(channel);
    let t = MyTest {
        test_client: Some(client),
    };
    let request = tonic::Request::new(StringRequest { str: format!("a") });
    eprintln!("{:?}", block_on(t.test_string(request)));
}
