/// Convert a SystemTime to a protobuf timestamp. Not sure about the correctness of nanoseconds calculation
pub fn systime_to_timestamp(t: std::time::SystemTime) -> prost_types::Timestamp {
    let nano_duration = t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let seconds = (nano_duration / 1_000_000_000) as i64;
    let nanos = (nano_duration % 1_000_000_000) as i32; // TODO Check correctness of this
    prost_types::Timestamp { seconds, nanos }
}

/// Convert a protobuf timestamp to a Rust SystemTime
pub fn timestamp_to_systime(t: prost_types::Timestamp) -> std::time::SystemTime {
    let nano_duration = t.nanos as u64 + t.seconds as u64 * 1_000_000_000;
    std::time::UNIX_EPOCH + std::time::Duration::from_nanos(nano_duration)
}
