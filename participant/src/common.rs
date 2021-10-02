use protos::service::contest;
use rocket::serde::Serialize;
use std::time::Duration;
use std::time::SystemTime;

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct Problem {
    pub id: u64,
    pub name: String,
}
impl From<contest::Problem> for Problem {
    fn from(p: contest::Problem) -> Self {
        Self {
            id: p.id,
            name: p.name,
        }
    }
}

#[derive(Serialize, Clone)]
#[serde(crate = "rocket::serde")]
pub struct ContestMetadata {
    pub name: String,
    pub start_time: String, // millis from unix epoch
    pub end_time: String,
    pub problems: Vec<Problem>,
}
impl From<contest::ContestMetadata> for ContestMetadata {
    fn from(c: contest::ContestMetadata) -> Self {
        let now = SystemTime::now();
        let is_running = match (c.start_time.clone(), c.end_time.clone()) {
            (Some(start_time), Some(end_time)) => {
                now >= SystemTime::from(start_time) && now < SystemTime::from(end_time)
            }
            _ => false,
        };
        Self {
            name: c.name.clone(),
            start_time: SystemTime::from(
                c.start_time
                    .unwrap_or_else(|| (now + Duration::from_secs(60)).into()),
            )
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string(),
            end_time: SystemTime::from(
                c.end_time
                    .unwrap_or_else(|| (now + Duration::from_secs(60 * 60 * 24)).into()),
            )
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
            .to_string(),
            problems: if is_running {
                vec![Problem {
                    id: 42,
                    name: String::from("problem ei"),
                }]
            } else {
                vec![]
            },
        }
    }
}
