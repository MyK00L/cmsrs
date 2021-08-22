use super::*;

#[test]
fn convert_to_mongo_timestamp_test_and_back() {
    let now = std::time::UNIX_EPOCH
        + std::time::Duration::from_secs(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        ); //This is needed because we need to discard anything below the second
    let mongo_now = mongo::systime_to_timestamp(now);
    assert_eq!(now, mongo::timestamp_to_systime(mongo_now));
}
#[test]
fn convert_to_protobuf_timestamp_test_and_back() {
    let now = std::time::SystemTime::now();
    let proto_now = proto::systime_to_timestamp(now);
    assert_eq!(now, proto::timestamp_to_systime(proto_now));
}
