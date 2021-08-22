use super::*;

fn get_test_time() -> std::time::SystemTime {
    std::time::UNIX_EPOCH
        + std::time::Duration::from_secs(10)
        + std::time::Duration::from_nanos(101)
}
#[test]
fn convert_to_mongo_timestamp_and_back_test() {
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
fn convert_to_protobuf_timestamp_and_back_test() {
    let now = std::time::SystemTime::now();
    let proto_now = proto::systime_to_timestamp(now);
    assert_eq!(now, proto::timestamp_to_systime(proto_now));
}

#[test]
fn convert_to_protobuf_timestamp_test() {
    let test_time = get_test_time();
    let proto_test_time = proto::systime_to_timestamp(test_time);
    assert_eq!(proto_test_time.seconds, 10);
    assert_eq!(proto_test_time.nanos, 101);
}

#[test]
fn convert_to_mongo_timestamp_test() {
    let test_time = get_test_time();
    let mongo_test_time = mongo::systime_to_timestamp(test_time);
    assert_eq!(mongo_test_time.time, 10);
    assert_eq!(mongo_test_time.increment, 0);
}
#[test]
fn convert_from_protobuf_timestamp_test() {
    let test_time = get_test_time();
    let proto_test_time = prost_types::Timestamp {
        seconds: 10,
        nanos: 101,
    };
    assert_eq!(proto::timestamp_to_systime(proto_test_time), test_time);
}

#[test]
fn convert_from_mongo_timestamp_test() {
    let test_time = get_test_time() - std::time::Duration::from_nanos(101);
    let mongo_test_time = bson::Timestamp {
        time: 10,
        increment: 0,
    };
    assert_eq!(mongo::timestamp_to_systime(mongo_test_time), test_time);
}
