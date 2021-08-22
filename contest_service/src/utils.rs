/// Convert MongoDB timestamps to Rust's SystemTime
pub fn timestamp_to_systime(ts: mongodb::bson::Timestamp) -> std::time::SystemTime {
    std::time::UNIX_EPOCH + std::time::Duration::from_secs(ts.time.into())
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
