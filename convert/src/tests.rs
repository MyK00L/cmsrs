use super::*;

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
    let epoch = std::time::UNIX_EPOCH;
    let proto_epoch = proto::systime_to_timestamp(epoch);
    assert_eq!(proto_epoch.seconds, 0);
    assert_eq!(proto_epoch.nanos, 0);
}

#[test]
fn convert_to_mongo_timestamp_test() {
    let epoch = std::time::UNIX_EPOCH;
    let mongo_epoch = mongo::systime_to_timestamp(epoch);
    assert_eq!(mongo_epoch.time, 0);
    assert_eq!(mongo_epoch.increment, 0);
}
#[test]
fn convert_from_protobuf_timestamp_test() {
    let epoch = std::time::UNIX_EPOCH;
    let proto_epoch = prost_types::Timestamp {
        seconds: 0,
        nanos: 0,
    };
    assert_eq!(proto::timestamp_to_systime(proto_epoch), epoch);
}

#[test]
fn convert_from_mongo_timestamp_test() {
    let epoch = std::time::UNIX_EPOCH;
    let mongo_epoch = bson::Timestamp {
        time: 0,
        increment: 0,
    };
    assert_eq!(mongo::timestamp_to_systime(mongo_epoch), epoch);
}
