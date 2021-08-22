/// Convert MongoDB timestamps to Rust's SystemTime
pub fn timestamp_to_systime(ts: mongodb::bson::Timestamp) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts.time.into())
}

pub fn systime_to_prost_ts(t: std::time::SystemTime) -> protos::prost_types::Timestamp {
    let nano_duration = t.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let seconds = (nano_duration / 1_000_000_000) as i64;
    let nanos = (nano_duration % 1_000_000_000) as i32; // TODO Check correctness of this
    protos::prost_types::Timestamp { seconds, nanos }
}
