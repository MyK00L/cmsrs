#![feature(duration_constants)]

use rand::thread_rng;
use rand::Rng;
use std::convert::TryInto;
use std::time::SystemTime;

pub mod mongo;
pub mod storage;

#[cfg(test)]
mod tests;

pub fn gen_uuid() -> u64 {
    ((thread_rng().gen::<u32>() as u64) << 32)
        | (((SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros()
            / 10)
            & 0xffffffff) as u64)
}

pub fn render_prost_timestamp(timestamp: prost_types::Timestamp, format_string: &str) -> String {
    let naive = chrono::prelude::NaiveDateTime::from_timestamp(
        timestamp.seconds,
        timestamp.nanos.try_into().unwrap_or(0),
    );
    let datetime = chrono::prelude::DateTime::<chrono::Utc>::from_utc(naive, chrono::Utc);
    datetime.format(format_string).to_string()
}
