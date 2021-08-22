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

    async fn get_contest_metadata(&self) -> Result<Document, Status> {
        Ok(self
            .get_contest_metadata_collection()
            .find_one(None, None)
            .await
            .map_err(|x| Status::internal(format!("{}", x)))?
            .ok_or_else(|| Status::not_found("Contest metadata not found"))?)
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
            .map_err(|x| Status::internal(format!("{}", x)))? // TODO fix with error conversion
            .filter(|x| {
                if let Some(hash_bson) = x.get("password") {
                    if let Some(hash) = hash_bson.as_str().to_owned() {
                        if let Ok(parsed_hash) = PasswordHash::new(hash) {
                            argon2::Argon2::default()
                                .verify_password(user.password.as_bytes(), &parsed_hash)
                                .is_ok()
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
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
            //.map_err(|x| Status::internal("Couldn't fetch contest from DB"))?
            .map(|x| mappings::contest::ContestMetadata::from(x).into())
            .map(|x| x)
    }
    async fn get_problem(
        &self,
        _request: Request<GetProblemRequest>,
    ) -> Result<Response<GetProblemResponse>, Status> {
        todo!();
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
            .map(|x| protos::user::Message::from(x))
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
            .map(|x| protos::user::Message::from(x))
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
        self.get_contest_metadata_collection()
            .delete_many(Document::new(), None)
            .await
            .map_err(internal_error)?; // This should delete every contest, since we don't want more than one

        let metadata = mappings::contest::ContestMetadata::try_from(request.into_inner())
            .map_err(|x| Status::invalid_argument(format!("{:?}", x)))?;

        self.get_contest_metadata_collection()
            .insert_one(Document::from(metadata), None)
            .await
            .map_err(internal_error)
            .map(|_| Response::new(SetContestMetadataResponse {}))
    }
    async fn set_problem(
        &self,
        _request: Request<SetProblemRequest>,
    ) -> Result<Response<SetProblemResponse>, Status> {
        todo!();
    }
    async fn add_question(
        &self,
        request: Request<AddQuestionRequest>,
    ) -> Result<Response<AddQuestionResponse>, Status> {
        let message = mappings::chat::Message::from(request.into_inner());
        // TODO should we notify someone here?
        let doc = Document::from(message);
        self.get_questions_collection()
            .insert_one(doc, None)
            .await
            .map_err(internal_error)?;
        Ok(Response::new(AddQuestionResponse {}))
    }
    async fn add_announcement(
        &self,
        request: Request<AddAnnouncementRequest>,
    ) -> Result<Response<AddAnnouncementResponse>, Status> {
        let message = mappings::chat::Message::from(request.into_inner());
        // TODO should we notify someone here?
        let doc = Document::from(message);
        self.get_announcements_collection()
            .insert_one(doc, None)
            .await
            .map_err(internal_error)?;
        Ok(Response::new(AddAnnouncementResponse {}))
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
