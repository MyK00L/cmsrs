use std::convert::TryFrom;

use mongodb::bson::Document;
use protos::service::contest::GetContestMetadataResponse;
use serde::{Deserialize, Serialize};
use tonic::Response;

#[derive(Debug)]
pub enum MappingError {
    MissingField(&'static str),
    PasswordAlreadyHashed,
    PasswordNotHashed,
    HashingError(argon2::password_hash::Error),
}

pub mod contest {
    use std::convert::TryInto;

    use super::*;

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ContestMetadata {
        name: String,
        description: String,
        start_time: Option<std::time::SystemTime>,
        end_time: Option<std::time::SystemTime>,
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

    #[derive(Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Message {
        #[serde(rename = "_id")]
        id: u32,
        subject: String,
        problem_id: Option<u32>,
        #[serde(rename = "text")]
        body: String,
        to: Option<String>,
        from: Option<String>,
        sent_at: std::time::SystemTime,
        #[serde(skip)]
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
        /*fn get_recipient(&self) -> Option<String> {
            if self.is_announcement() {
                self.to.clone()
            } else {
                self.from.clone()
            }
        }*/
    }

    impl From<protos::user::Message> for Message {
        fn from(msg: protos::user::Message) -> Self {
            Self {
                id: msg.id,
                subject: msg.subject,
                problem_id: msg.problem_id,
                body: msg.text,
                to: msg.to,
                from: msg.from,
                sent_at: msg
                    .sent_at
                    .map(|x| std::time::SystemTime::try_from(x).unwrap())
                    .unwrap(),
                _thread: None,
            }
        }
    }
    impl From<protos::service::contest::AddMessageRequest> for Message {
        fn from(req: protos::service::contest::AddMessageRequest) -> Self {
            Self::from(req.message.unwrap())
        }
    }
    impl From<Message> for protos::user::Message {
        fn from(msg: Message) -> Self {
            Self {
                id: msg.id,
                problem_id: msg.problem_id,
                subject: msg.subject.clone(),
                text: msg.body.clone(),
                sent_at: Some(protos::prost_types::Timestamp::from(msg.sent_at)),
                from: msg.from,
                to: msg.to,
            }
        }
    }
}

pub mod problem {
    pub struct ProblemData(Problem, Vec<u8>);
    impl ProblemData {
        pub fn get_problem(&self) -> Problem {
            self.0.clone()
        }
        pub fn get_statement(&self) -> Vec<u8> {
            self.1.clone()
        }
    }
    use mongodb::bson::Document;

    /*#[derive(Default, Clone)]
    pub struct Resources {
        time_limit: std::time::Duration,
        /// Maximum memory usage, in bytes
        memory: u64,
    }*/

    #[derive(Default, Clone)]
    pub struct Problem {
        id: u32,
        name: String,
        description: String,
        /*testcases_per_subtask: Vec<u32>,
        runtime_limits: Resources,
        source_size_limit: u64,
        task_type: String,*/
    }

    /*impl From<protos::common::Resources> for Resources {
        fn from(input: protos::common::Resources) -> Self {
            Resources {
                time_limit: input
                    .time
                    .map(|x| std::convert::TryInto::try_into(x).unwrap_or_default())
                    .unwrap(),
                memory: input.memory_bytes,
            }
        }
    }*/
    impl From<protos::user::Problem> for Problem {
        fn from(input: protos::user::Problem) -> Self {
            Problem {
                id: input.id,
                name: input.name,
                description: input.description,
                /*testcases_per_subtask: input.testcases_per_subtask,
                runtime_limits: input.runtime_limits.map(|x| x.into()).unwrap(),
                source_size_limit: input.source_size_limit,
                task_type: input.r#type,*/
            }
        }
    }
    /*impl From<Resources> for protos::common::Resources {
        fn from(input: Resources) -> Self {
            protos::common::Resources {
                time: Some(input.time_limit.into()),
                memory_bytes: input.memory,
            }
        }
    }*/
    impl From<Problem> for protos::user::Problem {
        fn from(p: Problem) -> Self {
            protos::user::Problem {
                id: p.id,
                name: p.name,
                description: p.description,
                /*testcases_per_subtask: p.testcases_per_subtask,
                runtime_limits: Some(p.runtime_limits.into()),
                source_size_limit: p.source_size_limit,
                r#type: p.task_type,*/
            }
        }
    }
    impl From<Document> for ProblemData {
        fn from(mongo_record: Document) -> Self {
            ProblemData(
                Problem {
                    id: mongo_record.get_i32("_id").unwrap_or_default() as u32,
                    name: mongo_record.get_str("name").unwrap_or_default().to_owned(),
                    description: mongo_record
                        .get_str("longName")
                        .unwrap_or_default()
                        .to_owned(),
                    //..Default::default()
                },
                mongo_record
                    .get_binary_generic("statement")
                    .unwrap()
                    .clone(),
            )
        }
    }
    impl From<ProblemData> for Document {
        fn from(problem_data: ProblemData) -> Self {
            let p = problem_data.get_problem();
            let statement = problem_data.get_statement();
            let mut result = Document::new();
            result.insert("_id", p.id);
            result.insert("name", p.name.clone());
            result.insert("longName", p.description);
            result.insert(
                "statement",
                mongodb::bson::Binary {
                    subtype: mongodb::bson::spec::BinarySubtype::Generic,
                    bytes: statement,
                },
            );
            result
        }
    }

    impl From<(Problem, Vec<u8>)> for ProblemData {
        fn from((p, bin): (Problem, Vec<u8>)) -> Self {
            ProblemData(p, bin)
        }
    }
}

pub mod user {
    use super::*;
    use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier};

    #[derive(Clone, Serialize, Deserialize)]
    enum Password {
        Hashed(String),
        Clear(String),
    }
    #[derive(Clone, Serialize, Deserialize)]
    pub struct User {
        username: String,
        name: String,
        password: Password,
    }

    impl User {
        fn hash_password(&mut self) -> Result<(), MappingError> {
            if let Password::Clear(pass) = &self.password {
                let argon2 = argon2::Argon2::default();

                let salt = argon2::password_hash::SaltString::generate(&mut rand_core::OsRng);
                let hashed_password = argon2
                    .hash_password_simple(pass.as_bytes(), &salt)
                    .map_err(MappingError::HashingError)?;
                self.password = Password::Hashed(hashed_password.to_string());
                Ok(())
            } else {
                Ok(())
            }
        }
        pub fn verify_password(&mut self, password: &str) -> Result<bool, MappingError> {
            if let Password::Hashed(hash) = &self.password {
                let hash = PasswordHash::new(hash).map_err(MappingError::HashingError)?;
                Ok(argon2::Argon2::default()
                    .verify_password(password.as_bytes(), &hash)
                    .is_ok())
            } else {
                Err(MappingError::PasswordNotHashed)
            }
        }
        pub fn get_username(&self) -> &str {
            &self.username
        }
    }

    impl From<Document> for User {
        fn from(record: Document) -> Self {
            Self {
                username: record.get_str("_id").unwrap().to_owned(),
                name: record.get_str("fullName").unwrap().to_owned(),
                // Here I assume that the password stored in the DB are hashed, since I hash them before insertion
                password: Password::Hashed(record.get_str("password").unwrap().to_owned()),
            }
        }
    }
    impl From<protos::service::contest::SetUserRequest> for User {
        fn from(pb: protos::service::contest::SetUserRequest) -> Self {
            Self {
                username: pb.username,
                name: pb.fullname,
                password: Password::Clear(pb.password),
            }
        }
    }
    impl From<User> for Document {
        fn from(u: User) -> Self {
            let mut u = u;
            u.hash_password().expect("Could not hash password");
            let mut result = Document::new();
            result.insert("_id", u.username);
            result.insert("longName", u.name);
            result.insert(
                "password",
                {
                    match u.password {
                        Password::Clear(_) => None,
                        Password::Hashed(h) => Some(h),
                    }
                }
                .unwrap(),
            );
            result
        }
    }
    impl From<User> for protos::service::contest::auth_user_response::Response {
        fn from(u: User) -> Self {
            Self::Success(protos::service::contest::auth_user_response::Success {
                username: u.username,
                fullname: u.name,
            })
        }
    }
}
