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
        let inner = request.into_inner();
        eprintln!("MockTest.test_string({:?})", inner);
        Ok(Response::new(StringResponse {
            str: format!("Mock {}", inner.str),
        }))
    }
    async fn log_string(
        &self,
        request: tonic::Request<LogRequest>,
    ) -> Result<tonic::Response<LogResponse>, tonic::Status> {
        let inner = request.into_inner();
        eprintln!("MockTest.log_string({:?})", inner);
        Ok(Response::new(LogResponse {}))
    }
}

#[test]
fn string_test() {
    let channel = ServiceBuilder::new().service(TestServer::new(MockTest::default()));
    let test_client = TestClient::new(channel);
    let t = MyTest { test_client };
    let request = tonic::Request::new(StringRequest { str: format!("42") });
    assert_eq!(
        StringResponse {
            str: String::from("Hello 42")
        },
        block_on(t.test_string(request)).unwrap().into_inner()
    );
}
