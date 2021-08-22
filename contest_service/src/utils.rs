/// Convert MongoDB timestamps to Rust's SystemTime
pub fn timestamp_to_systime(ts: mongodb::bson::Timestamp) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts.time.into())
}

/// Convert a SystemTime to a protobuf timestamp. Not sure about the correctness of nanoseconds calculation
pub fn systime_to_prost_ts(t: std::time::SystemTime) -> protos::prost_types::Timestamp {
    let nano_duration = t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let seconds = (nano_duration / 1_000_000_000) as i64;
    let nanos = (nano_duration % 1_000_000_000) as i32; // TODO Check correctness of this
    protos::prost_types::Timestamp { seconds, nanos }
}

/// Convert a protobuf timestamp to a Rust SystemTime
pub fn prost_ts_to_systime(t: protos::prost_types::Timestamp) -> std::time::SystemTime {
    let nano_duration = t.nanos as u64 + t.seconds as u64 * 1_000_000_000;
    std::time::UNIX_EPOCH + std::time::Duration::from_nanos(nano_duration)
}

/// Convert a SystemTime to a MongoDB timestamp. Note that MongoDB sucks and stores unix timestamps in 32 bits
pub fn systime_to_timestamp(st: std::time::SystemTime) -> mongodb::bson::Timestamp {
    let nano_duration = st.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let seconds = (nano_duration / 1_000_000_000) as u32; // TODO This is bad, because we will have an overflow in 2106
    mongodb::bson::Timestamp {
        time: seconds,
        increment: 0,
    }
}
