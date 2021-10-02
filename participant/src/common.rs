use protos::service::contest;
use rocket::serde::Serialize;
use std::time::Duration;
use std::time::SystemTime;

//use std::convert::TryInto;
/*fn render_timestamp(timestamp: protos::common::Timestamp, format_string: &str) -> String {
    let naive = chrono::prelude::NaiveDateTime::from_timestamp(
        timestamp.secs.try_into().unwrap_or_default(),
        timestamp.nanos,
    );
    let datetime = chrono::prelude::DateTime::<chrono::Utc>::from_utc(naive, chrono::Utc);
    datetime.format(format_string).to_string()
}*/

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct Problem {
    id: u64,
    name: String,
}
impl From<contest::Problem> for Problem {
    fn from(p: contest::Problem) -> Self {
        Self {
            id: p.id,
            name: p.name,
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ContestMetadata {
    name: String,
    start_time: String, // millis from unix epoch
    end_time: String,
    problems: Vec<Problem>,
}
impl From<contest::ContestMetadata> for ContestMetadata {
    fn from(c: contest::ContestMetadata) -> Self {
        Self {
            name: c.name.clone(),
            start_time: SystemTime::from(
                c.start_time.unwrap_or_else(|| {
                    (SystemTime::now() + Duration::from_secs(60 * 60 * 24)).into()
                }),
            )
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis().to_string(),
            end_time: SystemTime::from(
                c.end_time.unwrap_or_else(|| {
                    (SystemTime::now() + Duration::from_secs(60 * 60 * 24)).into()
                }),
            )
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis().to_string(),
            problems: vec![],
        }
    }
}
