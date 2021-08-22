use std::convert::TryFrom;

use mongodb::bson::Document;
use protos::service::contest::GetContestMetadataResponse;
use tonic::Response;

#[derive(Debug)]
pub enum MappingError {
    MissingField(&'static str),
}

pub mod contest {
    use std::convert::TryInto;

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
                start_time: value.get("startTime").map(|x| {
                    x.as_timestamp()
                        .map(convert::mongo::timestamp_to_systime)
                        .unwrap()
                }),
                end_time: value.get("endTime").map(|x| {
                    x.as_timestamp()
                        .map(convert::mongo::timestamp_to_systime)
                        .unwrap()
                }),
            }
        }
    }
    impl From<ContestMetadata> for Response<GetContestMetadataResponse> {
        fn from(md: ContestMetadata) -> Self {
            Response::new(GetContestMetadataResponse {
                metadata: Some(protos::service::contest::ContestMetadata {
                    name: md.name,
                    description: md.description,
                    start_time: md
                        .start_time
                        .map(|x| protos::prost_types::Timestamp::try_from(x).unwrap()), // This should not break,
                    end_time: md
                        .end_time
                        .map(|x| protos::prost_types::Timestamp::try_from(x).unwrap()), // This should not break,
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
                start_time: metadata.start_time.map(|x| x.try_into().unwrap()),
                end_time: metadata.end_time.map(|x| x.try_into().unwrap()),
            })
        }

        type Error = MappingError;
    }

    impl From<ContestMetadata> for Document {
        fn from(m: ContestMetadata) -> Self {
            let mut result = Document::new();
            result.insert("name", m.name);
            result.insert("description", m.description);
            result.insert(
                "startTime",
                m.start_time.map(convert::mongo::systime_to_timestamp),
            );
            result.insert(
                "endTime",
                m.end_time.map(convert::mongo::systime_to_timestamp),
            );
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
        pub fn is_announcement(&self) -> bool {
            self.from.is_none()
        }
        pub fn is_broadcast(&self) -> bool {
            self.is_announcement() && self.to.is_none()
        }
        pub fn is_question(&self) -> bool {
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
                    created: msg
                        .timestamp
                        .map(|x| std::time::SystemTime::try_from(x).unwrap())
                        .unwrap(),
                    _thread: None,
                },
                MessageType::Question => Self {
                    id: msg.id,
                    subject: msg.subject,
                    problem_id: msg.problem_id,
                    body: msg.text,
                    to: None,
                    from: msg.user,
                    created: msg
                        .timestamp
                        .map(|x| std::time::SystemTime::try_from(x).unwrap())
                        .unwrap(),
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
                timestamp: Some(protos::prost_types::Timestamp::from(msg.created)),
                user: msg.get_recipient(),
            }
        }
    }
    impl From<Message> for mongodb::bson::Document {
        fn from(m: Message) -> Self {
            let mut resp = Document::new();
            resp.insert("_id", m.id);
            resp.insert("subject", m.subject.clone());
            resp.insert("problemId", m.problem_id);
            resp.insert("text", m.body.clone());
            if m.is_announcement() {
                resp.insert("to", m.get_recipient());
            } else if m.is_question() {
                resp.insert("from", m.get_recipient());
            }
            resp.insert("created", convert::mongo::systime_to_timestamp(m.created));
            resp
        }
    }
    impl From<Document> for Message {
        fn from(d: Document) -> Self {
            Self {
                id: d.get_i32("_id").unwrap() as u32,
                subject: d.get_str("subject").unwrap().to_owned(),
                problem_id: d.get_i32("problemId").map(|x| x as u32).ok(),
                body: d.get_str("text").unwrap().to_owned(),
                to: d.get_str("to").map(|x| x.to_owned()).ok(),
                from: d.get_str("from").map(|x| x.to_owned()).ok(),
                created: convert::mongo::timestamp_to_systime(d.get_timestamp("created").unwrap()),
                _thread: None,
            }
        }
    }
}
