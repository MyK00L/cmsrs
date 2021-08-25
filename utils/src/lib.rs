use rand::thread_rng;
use rand::Rng;
use std::time::SystemTime;

pub fn gen_uuid() -> u64 {
    ((thread_rng().gen::<u32>() as u64) << 32)
        | (((SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_micros()
            / 10)
            & 0xffffffff) as u64)
}
