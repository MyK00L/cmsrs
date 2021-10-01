use rocket::serde::Serialize;
use protos::service::contest;
use std::convert::TryInto;

fn render_timestamp(
    timestamp: protos::common::Timestamp,
    format_string: &str,
) -> String {
    let naive = chrono::prelude::NaiveDateTime::from_timestamp(
        timestamp.secs.try_into().unwrap_or_default(),
        timestamp.nanos,
    );
    let datetime = chrono::prelude::DateTime::<chrono::Utc>::from_utc(naive, chrono::Utc);
    datetime.format(format_string).to_string()
}

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
            name: p.name.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "rocket::serde")]
pub struct ContestMetadata {
    name: String,
    start_time: String,
    end_time: String,
    problems: Vec<Problem>,
}
impl From<contest::ContestMetadata> for ContestMetadata {
    fn from(c: contest::ContestMetadata) -> Self {
        Self {
            name: c.name.clone(),
            start_time: c.start_time.map(|x| render_timestamp(x, "%FT%T")).unwrap_or_default(),
            end_time: c.end_time.map(|x| render_timestamp(x, "%FT%T")).unwrap_or_default(),
            problems: vec![],
        }
    }
}



