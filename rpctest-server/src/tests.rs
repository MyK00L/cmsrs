use super::*;
use tokio_test::block_on;
use tower::ServiceBuilder;

#[test]
fn string_test() {
    let mut mock_test = MockTest::default();
    mock_test.test_string_set(TestStringResponse {
        str: String::from("wah"),
    });
    let channel = ServiceBuilder::new().service(TestServer::new(mock_test));
    let test_client = TestClient::new(channel);
    let t = MyTest { test_client };
    let request = tonic::Request::new(TestStringRequest { str: format!("42") });
    assert_eq!(
        TestStringResponse {
            str: String::from("Hello 42")
        },
        block_on(t.test_string(request)).unwrap().into_inner()
    );
}
