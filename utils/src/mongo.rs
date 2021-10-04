/// Convert MongoDB timestamps to Rust's SystemTime
pub fn timestamp_to_systime(ts: bson::Timestamp) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts.time.into())
}

/// Convert a SystemTime to a MongoDB timestamp. Note that MongoDB stores unix timestamps in 32 bits
pub fn systime_to_timestamp(st: std::time::SystemTime) -> bson::Timestamp {
    let nano_duration = st.duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
    let seconds = (nano_duration / std::time::Duration::SECOND.as_nanos()) as u32; // This will overflow in 2106
    bson::Timestamp {
        time: seconds,
        increment: 0,
    }
}

pub fn u64_to_i64(x: u64) -> i64 {
    i64::from_le_bytes(x.to_le_bytes())
}
pub fn i64_to_u64(x: i64) -> u64 {
    u64::from_le_bytes(x.to_le_bytes())
}
