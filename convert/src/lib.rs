//! Convertion utilities between protobuf and mongodb
pub mod mongo;
pub mod proto;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_test_mongo() {
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
    fn cycle_test_proto() {
        let now = std::time::SystemTime::now();
        let proto_now = proto::systime_to_timestamp(now);
        assert_eq!(now, proto::timestamp_to_systime(proto_now));
    }
}
