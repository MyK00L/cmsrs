use super::*;
use tokio_test::block_on;
use tower::ServiceBuilder;
use tonic::Request;

#[test]
fn string_test() {
    let mut mock_test = MockTest::default();
    mock_test.test_string_set(TestStringResponse {
        str: String::from("wah"),
    });
    mock_test.log_string_set(LogStringResponse {});
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

#[test]
#[should_panic]
fn mock_with_err() {
    let mut mock_test = MockTest::default();
    mock_test.test_string_set_err(tonic::Status::unimplemented("o.O"));
    let channel = ServiceBuilder::new().service(TestServer::new(mock_test));
    let test_client = TestClient::new(channel);
    let t = MyTest { test_client };
    let request = tonic::Request::new(TestStringRequest { str: format!("42") });
    eprintln!(
        "{:?}",
        block_on(t.test_string(request)).unwrap().into_inner()
    );
}

async fn async_file_test() {
    let mut mock_test = MockTest::default();
    let messages = vec![
        FileRequest {file: (b"owo").to_vec(),},
        FileRequest {file: (b"ouo").to_vec(),},
        FileRequest {file: (b"ovo").to_vec(),},
    ];
    let channel = ServiceBuilder::new().service(TestServer::new(mock_test));
    let test_client = TestClient::new(channel);
    let t = MyTest { test_client };
    let mut streaming_response = t.file(tonic::Request::new(futures_util::stream::iter(messages.clone()))).await.unwrap();
    /*while let Some(res) = block_on(streaming_response.message()).unwrap() {
        println!("{:?}", res);
    }*/
}

#[test]
fn file_test() {
    block_on(async_file_test());
}
