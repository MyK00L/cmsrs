use std::convert::TryFrom;

use super::utils;
use mongodb::bson::Document;
use protos::service::contest::GetContestMetadataResponse;
use tonic::Response;

#[derive(Debug)]
pub enum MappingError {
    MissingField(&'static str),
}

pub mod contest {
    use super::*;
    pub struct ContestMetadata {
        name: String,
        description: String,
        start_time: Option<std::time::SystemTime>,
        end_time: Option<std::time::SystemTime>,
    }
    impl From<Document> for ContestMetadata {
        fn from(value: Document) -> Self {
            Self {
                name: value.get("name").unwrap().to_string(),
                description: value.get("description").unwrap().to_string(),
                start_time: value
                    .get("startTime")
                    .map(|x| x.as_timestamp().map(utils::timestamp_to_systime).unwrap()),
                end_time: value
                    .get("endTime")
                    .map(|x| x.as_timestamp().map(utils::timestamp_to_systime).unwrap()),
            }
        }
    }
    impl From<ContestMetadata> for Response<GetContestMetadataResponse> {
        fn from(md: ContestMetadata) -> Self {
            Response::new(GetContestMetadataResponse {
                metadata: Some(protos::service::contest::ContestMetadata {
                    name: md.name,
                    description: md.description,
                    start_time: md.start_time.map(utils::systime_to_prost_ts),
                    end_time: md.end_time.map(utils::systime_to_prost_ts),
                }),
            })
        }
    }

    impl TryFrom<protos::service::contest::SetContestMetadataRequest> for ContestMetadata {
        fn try_from(
            pb_meta: protos::service::contest::SetContestMetadataRequest,
        ) -> Result<Self, Self::Error> {
            let metadata = pb_meta
                .metadata
                .ok_or(MappingError::MissingField("metadata"))?;
            Ok(Self {
                name: metadata.name,
                description: metadata.description,
                start_time: metadata.start_time.map(utils::prost_ts_to_systime),
                end_time: metadata.end_time.map(utils::prost_ts_to_systime),
            })
        }

        type Error = MappingError;
    }

    impl From<ContestMetadata> for Document {
        fn from(m: ContestMetadata) -> Self {
            let mut result = Document::new();
            result.insert("name", m.name);
            result.insert("description", m.description);
            result.insert("startTime", m.start_time.map(utils::systime_to_timestamp));
            result.insert("endTime", m.end_time.map(utils::systime_to_timestamp));
            result
        }
    }
}

pub mod chat {
    use super::*;
    pub struct Message {
        id: u32,
        subject: String,
        problem_id: Option<u32>,
        body: String,
        to: Option<String>,
        from: Option<String>,
        created: std::time::SystemTime,
        _thread: Option<i64>,
    }
    impl Message {
        fn is_announcement(&self) -> bool {
            self.from.is_none()
        }
        fn is_broadcast(&self) -> bool {
            self.is_announcement() && self.to.is_none()
        }
        fn is_question(&self) -> bool {
            self.to.is_none() && self.from.is_some()
        }
        fn get_recipient(&self) -> Option<String> {
            if self.is_announcement() {
                self.to.clone()
            } else {
                self.from.clone()
            }
        }
    }

    enum MessageType {
        Announcement,
        Question,
    }

    impl From<(protos::user::Message, MessageType)> for Message {
        fn from((msg, msg_type): (protos::user::Message, MessageType)) -> Self {
            match msg_type {
                MessageType::Announcement => Self {
                    id: msg.id,
                    subject: msg.subject,
                    problem_id: msg.problem_id,
                    body: msg.text,
                    to: msg.user,
                    from: None,
                    created: msg.timestamp.map(utils::prost_ts_to_systime).unwrap(),
                    _thread: None,
                },
                MessageType::Question => Self {
                    id: msg.id,
                    subject: msg.subject,
                    problem_id: msg.problem_id,
                    body: msg.text,
                    to: None,
                    from: msg.user,
                    created: msg.timestamp.map(utils::prost_ts_to_systime).unwrap(),
                    _thread: None,
                },
            }
        }
    }
    impl From<protos::service::contest::AddQuestionRequest> for Message {
        fn from(req: protos::service::contest::AddQuestionRequest) -> Self {
            Self::from((req.question.unwrap(), MessageType::Question))
        }
    }
    impl From<protos::service::contest::AddAnnouncementRequest> for Message {
        fn from(req: protos::service::contest::AddAnnouncementRequest) -> Self {
            Self::from((req.announcement.unwrap(), MessageType::Announcement))
        }
    }
    impl From<Message> for protos::user::Message {
        fn from(msg: Message) -> Self {
            Self {
                id: msg.id,
                problem_id: msg.problem_id,
                subject: msg.subject.clone(),
                text: msg.body.clone(),
                timestamp: Some(utils::systime_to_prost_ts(msg.created)),
                user: msg.get_recipient(),
            }
        }
    }
    impl From<Message> for mongodb::bson::Document {
        fn from(m: Message) -> Self {
            let mut resp = Document::new();
            resp.insert("id", m.id);
            resp.insert("subject", m.subject);
            resp.insert("problemId", m.problem_id);
            resp.insert("text", m.body);
            if m.is_announcement() {
                resp.insert("to", m.get_recipient());
            } else {
                resp.insert("from", m.get_recipient());
            }
            resp.insert("created", m.created);
            resp
        }
    }
}
