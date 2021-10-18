use std::convert::TryFrom;

use mongodb::bson::Document;

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
    pub struct ContestMetadata {
        name: String,
        description: String,
        start_time: Option<std::time::SystemTime>,
        end_time: Option<std::time::SystemTime>,
    }
    impl Default for ContestMetadata {
        fn default() -> Self {
            Self {
                name: String::from("contest"),
                description: String::from("no description"),
                ..Default::default()
            }
        }
    }
    impl From<Document> for ContestMetadata {
        fn from(value: Document) -> Self {
            Self {
                name: value.get("name").unwrap().to_string(),
                description: value.get("description").unwrap().to_string(),
                start_time: value.get("startTime").map(|x| {
                    x.as_timestamp()
                        .map(utils::mongo::timestamp_to_systime)
                        .unwrap()
                }),
                end_time: value.get("endTime").map(|x| {
                    x.as_timestamp()
                        .map(utils::mongo::timestamp_to_systime)
                        .unwrap()
                }),
            }
        }
    }
    impl From<ContestMetadata> for protos::service::contest::ContestMetadata {
        fn from(md: ContestMetadata) -> Self {
            protos::service::contest::ContestMetadata {
                name: md.name,
                description: md.description,
                start_time: md.start_time.map(protos::common::Timestamp::from),
                end_time: md.end_time.map(protos::common::Timestamp::from),
            }
        }
    }

    impl TryFrom<protos::service::contest::SetContestMetadataRequest> for ContestMetadata {
        fn try_from(
            pb_meta: protos::service::contest::SetContestMetadataRequest,
        ) -> Result<Self, Self::Error> {
            let metadata = pb_meta.metadata;
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
                m.start_time.map(utils::mongo::systime_to_timestamp),
            );
            result.insert(
                "endTime",
                m.end_time.map(utils::mongo::systime_to_timestamp),
            );
            result
        }
    }
}

pub mod chat {
    use super::*;

    pub struct Message {
        id: u64,
        subject: String,
        problem_id: Option<u64>,
        body: String,
        to: Option<String>,
        from: Option<String>,
        sent_at: std::time::SystemTime,
        //#[serde(skip)]
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

    impl From<protos::service::contest::Message> for Message {
        fn from(msg: protos::service::contest::Message) -> Self {
            Self {
                id: msg.id,
                subject: msg.subject,
                problem_id: msg.problem_id,
                body: msg.text,
                to: msg.to,
                from: msg.from,
                sent_at: std::time::SystemTime::try_from(msg.sent_at).unwrap(),
                _thread: None,
            }
        }
    }
    impl From<protos::service::contest::AddMessageRequest> for Message {
        fn from(req: protos::service::contest::AddMessageRequest) -> Self {
            Self::from(req.message)
        }
    }
    impl From<Message> for protos::service::contest::Message {
        fn from(msg: Message) -> Self {
            Self {
                id: msg.id,
                problem_id: msg.problem_id,
                subject: msg.subject.clone(),
                text: msg.body.clone(),
                sent_at: protos::common::Timestamp::from(msg.sent_at),
                from: msg.from,
                to: msg.to,
            }
        }
    }
    impl From<Message> for mongodb::bson::Document {
        fn from(m: Message) -> Self {
            let mut resp = Document::new();
            resp.insert("_id", m.id as i64);
            resp.insert("subject", m.subject.clone());
            resp.insert("problemId", m.problem_id.map(|id| id as i64));
            resp.insert("text", m.body.clone());
            if m.is_announcement() {
                resp.insert("to", m.get_recipient());
            } else if m.is_question() {
                resp.insert("from", m.get_recipient());
            }
            resp.insert("created", utils::mongo::systime_to_timestamp(m.sent_at));
            resp
        }
    }
    impl From<Document> for Message {
        fn from(d: Document) -> Self {
            Self {
                id: d.get_i64("_id").unwrap() as u64,
                subject: d.get_str("subject").unwrap().to_owned(),
                problem_id: d.get_i64("problemId").map(|x| x as u64).ok(),
                body: d.get_str("text").unwrap().to_owned(),
                to: d.get_str("to").map(|x| x.to_owned()).ok(),
                from: d.get_str("from").map(|x| x.to_owned()).ok(),
                sent_at: utils::mongo::timestamp_to_systime(d.get_timestamp("created").unwrap()),
                _thread: None,
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

    #[derive(Default, Clone)]
    pub struct Problem {
        id: u64,
        pub name: String,
        pub long_name: String,
    }

    impl Problem {
        pub fn get_id(&self) -> i64 {
            self.id as i64
        }
    }

    impl From<protos::service::contest::Problem> for Problem {
        fn from(input: protos::service::contest::Problem) -> Self {
            Problem {
                id: input.id,
                name: input.name,
                long_name: input.long_name,
            }
        }
    }

    impl From<Problem> for protos::service::contest::Problem {
        fn from(p: Problem) -> Self {
            protos::service::contest::Problem {
                id: p.id,
                name: p.name,
                long_name: p.long_name,
            }
        }
    }
    impl From<Document> for ProblemData {
        fn from(mongo_record: Document) -> Self {
            ProblemData(
                Problem {
                    id: mongo_record.get_i64("_id").unwrap_or_default() as u64,
                    name: mongo_record.get_str("name").unwrap_or_default().to_owned(),
                    long_name: mongo_record
                        .get_str("longName")
                        .unwrap_or_default()
                        .to_owned(),
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
            result.insert("_id", p.id as i64);
            result.insert("name", p.name.clone());
            result.insert("longName", p.long_name);
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

    #[derive(Clone)]
    enum Password {
        Hashed(String),
        Clear(String),
    }
    #[derive(Clone)]
    pub struct User {
        username: String,
        fullname: String,
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
                fullname: record.get_str("fullname").unwrap().to_owned(),
                // Here I assume that the password stored in the DB are hashed, since I hash them before insertion
                password: Password::Hashed(record.get_str("password").unwrap().to_owned()),
            }
        }
    }
    impl From<protos::service::contest::SetUserRequest> for User {
        fn from(pb: protos::service::contest::SetUserRequest) -> Self {
            Self {
                username: pb.username,
                fullname: pb.fullname,
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
            result.insert("fullname", u.fullname);
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
                fullname: u.fullname,
            })
        }
    }
}
