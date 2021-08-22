/// Convert MongoDB timestamps to Rust's SystemTime
pub fn timestamp_to_instant(ts: &mongodb::bson::Timestamp) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts.time.into())
}
