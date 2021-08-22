use futures::stream::StreamExt;
use std::convert::TryFrom;

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier};
use mappings::chat::Message;
use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, UpdateOptions},
    Client,
};
use protos::service::contest::{contest_server::*, *};
use protos::utils::*;
use tonic::{transport::*, Request, Response, Status};

mod mappings;

#[cfg(test)]
mod tests;

// TODO: remove credentials to connect to db.
const CONNECTION_STRING: &str = "mongodb://root:example@contest_service_db:27017/";

fn internal_error<T>(e: T) -> Status
where
    T: std::fmt::Debug,
{
    Status::internal(format!("{:?}", e))
}

#[derive(Debug)]
pub struct ContestService {
    db_client: Client,
}

impl ContestService {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            db_client: Client::with_options(ClientOptions::parse(CONNECTION_STRING).await?)?,
        })
    }

    /// Do not call this function, call get_*_collection or get_contest_metadata instead
    fn get_collection(&self, collection_name: &str) -> mongodb::Collection<Document> {
        let db = self.db_client.database("contestdb");
        db.collection::<Document>(collection_name)
    }

    fn get_contest_metadata_collection(&self) -> mongodb::Collection<Document> {
        self.get_collection("contest_metadata")
    }

    fn get_problems_collection(&self) -> mongodb::Collection<Document> {
        self.get_collection("problems")
    }

    fn get_users_collection(&self) -> mongodb::Collection<Document> {
        self.get_collection("users")
    }

    fn get_announcements_collection(&self) -> mongodb::Collection<Document> {
        self.get_collection("announcements")
    }

    fn get_questions_collection(&self) -> mongodb::Collection<Document> {
        self.get_collection("questions")
    }

    async fn get_contest_metadata(&self) -> Result<Document, Status> {
        Ok(self
            .get_contest_metadata_collection()
            .find_one(None, None)
            .await
            .map_err(|x| Status::internal(format!("{}", x)))?
            .ok_or_else(|| Status::not_found("Contest metadata not found"))?)
    }
}

fn check_password(password: &str, user_doc: &Document) -> bool {
    user_doc.get_str("password").map_or(false, |hash| {
        PasswordHash::new(hash).map_or(false, |parsed_hash| {
            argon2::Argon2::default()
                .verify_password(password.as_bytes(), &parsed_hash)
                .is_ok()
        })
    })
}

#[tonic::async_trait]
impl Contest for ContestService {
    async fn auth_user(
        &self,
        request: Request<AuthUserRequest>,
    ) -> Result<Response<AuthUserResponse>, Status> {
        let user = request.into_inner();
        let username = user.username.clone();
        Ok(self
            .get_users_collection()
            .find_one(
                doc! {
                    "_id": username,
                },
                None,
            )
            .await
            .map_err(|err| Status::internal(format!("{}", err)))? // TODO fix with error conversion
            .filter(|user_doc| check_password(&user.password, user_doc))
            .map_or(
                Response::new(AuthUserResponse {
                    response: Some(auth_user_response::Response::Failure(
                        auth_user_response::Failure {
                            error: "Incorrect username or password".to_string(),
                        },
                    )),
                }),
                |user_doc| {
                    Response::new(AuthUserResponse {
                        response: Some(auth_user_response::Response::Success(
                            auth_user_response::Success {
                                username: user_doc.get("_id").unwrap().to_string(),
                                fullname: user_doc.get("fullName").unwrap().to_string(),
                            },
                        )),
                    })
                },
            ))
    }
    async fn get_contest_metadata(
        &self,
        _request: Request<GetContestMetadataRequest>,
    ) -> Result<Response<GetContestMetadataResponse>, Status> {
        self.get_contest_metadata()
            .await
            .map(|contest_metadata_doc| {
                mappings::contest::ContestMetadata::from(contest_metadata_doc).into()
            })
    }
    async fn get_problem(
        &self,
        request: Request<GetProblemRequest>,
    ) -> Result<Response<GetProblemResponse>, Status> {
        let problem_id = request.into_inner().problem_id;
        self.get_problems_collection()
            .find_one(doc! {"_id": problem_id}, None)
            .await
            .map_err(internal_error)?
            .map(mappings::problem::ProblemData::from)
            .map(|x| {
                Response::new(GetProblemResponse {
                    info: Some(x.get_problem().into()),
                    statement: x.get_statement(),
                })
            })
            .ok_or_else(|| Status::internal("Problem not found"))
    }
    async fn get_announcement_list(
        &self,
        _request: Request<GetAnnouncementListRequest>,
    ) -> Result<Response<GetAnnouncementListResponse>, Status> {
        let announcements = self
            .get_announcements_collection()
            .find(None, None)
            .await
            .map_err(internal_error)?
            .map(|x| Message::from(x.unwrap()))
            .map(|x| x.into())
            .collect::<Vec<_>>()
            .await;
        Ok(Response::new(GetAnnouncementListResponse { announcements }))
    }
    async fn get_question_list(
        &self,
        _request: Request<GetQuestionListRequest>,
    ) -> Result<Response<GetQuestionListResponse>, Status> {
        let questions = self
            .get_questions_collection()
            .find(None, None)
            .await
            .map_err(internal_error)?
            .map(|x| Message::from(x.unwrap()))
            .map(|x| x.into())
            .collect::<Vec<_>>()
            .await;
        Ok(Response::new(GetQuestionListResponse { questions }))
    }
    async fn set_user(
        &self,
        request: Request<SetUserRequest>,
    ) -> Result<Response<SetUserResponse>, Status> {
        let argon2 = argon2::Argon2::default();

        let user = request.into_inner();
        let username = user.username.clone();
        let fullname = user.fullname.clone();

        let salt = argon2::password_hash::SaltString::generate(&mut rand_core::OsRng);
        let hashed_password = argon2
            .hash_password_simple(user.password.as_bytes(), &salt)
            .map_err(|_| tonic::Status::new(tonic::Code::Internal, "failed to hash password"))?;

        self.get_users_collection()
            .update_one(
                doc! {
                    "_id": username
                },
                doc! {
                    "$set": doc! {
                        "fullName": fullname,
                        "password": hashed_password.to_string(),
                    }
                },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
            .map_err(|err| Status::internal(format!("{}", err))) // TODO fix with error conversion
            .map(|update_result| {
                Response::new(SetUserResponse {
                    code: if update_result.matched_count == 0 {
                        set_user_response::Code::Add as i32
                    } else {
                        set_user_response::Code::Update as i32
                    },
                })
            })
    }
    async fn set_contest_metadata(
        &self,
        request: Request<SetContestMetadataRequest>,
    ) -> Result<Response<SetContestMetadataResponse>, Status> {
        let metadata = mappings::contest::ContestMetadata::try_from(request.into_inner())
            .map_err(|err| Status::invalid_argument(format!("{:?}", err)))?;

        self.get_contest_metadata_collection()
            .update_one(
                doc! {},
                doc! { "$set": Document::from(metadata) },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
            .map_err(internal_error)
            .map(|_| Response::new(SetContestMetadataResponse {}))
    }
    async fn set_problem(
        &self,
        request: Request<SetProblemRequest>,
    ) -> Result<Response<SetProblemResponse>, Status> {
        let problem_data_from_req = request.into_inner();
        if let Some(p) = problem_data_from_req.info {
            if let Some(bin) = problem_data_from_req.statement {
                let problem_data: mappings::problem::ProblemData = (p.into(), bin).into();

                let document: Document = problem_data.into();
                self.get_problems_collection()
                    .update_one(
                        doc! {"_id": document.get_i32("_id").unwrap()},
                        doc! { "$set": document },
                        UpdateOptions::builder().upsert(true).build(),
                    )
                    .await
                    .map_err(internal_error)
                    .map(|_| Response::new(SetProblemResponse {}))
            } else {
                Err(Status::invalid_argument("Missing required parameter"))
            }
        } else {
            Err(Status::invalid_argument("Missing required parameter"))
        }
    }
    async fn add_message(
        &self,
        request: Request<AddMessageRequest>,
    ) -> Result<Response<AddMessageResponse>, Status> {
        let message = mappings::chat::Message::from(request.into_inner());
        // TODO should we notify someone here?
        if message.is_question() {
            self.get_questions_collection()
        } else {
            self.get_announcements_collection()
        }
        .insert_one(Document::from(message), None)
        .await
        .map_err(internal_error)?;
        Ok(Response::new(AddMessageResponse {}))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = get_local_address(Service::CONTEST).parse()?;
    let contest_service = ContestService::new().await?;

    println!("Starting contest server");
    Server::builder()
        .add_service(ContestServer::new(contest_service))
        .serve(addr)
        .await?;
    Ok(())
}
