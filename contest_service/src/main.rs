use mongodb::{
    bson::{doc, Document},
    options::ClientOptions,
    Client,
};
use protos::service::contest::{contest_server::*, *};
use protos::utils::*;
use tonic::{transport::*, Request, Response, Status};

#[cfg(test)]
mod tests;

const CONNECTION_STRING: &str = "mongodb://root:example@contest_service_db:27017/";

#[derive(Debug)]
pub struct ContestService {
    client: Client,
}

impl ContestService {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            client: Client::with_options(ClientOptions::parse(CONNECTION_STRING).await?)?,
        })
    }
    fn get_contests_collection(&self) -> mongodb::Collection<Document> {
        let db = self.client.database("contestdb");
        db.collection::<Document>("contests")
    }
    async fn get_default_contest_db(&self) -> Result<Document, Status> {
        Ok(self
            .get_contests_collection()
            .find_one(None, None)
            .await
            .map_err(|x| Status::internal(format!("{}", x)))?
            .ok_or_else(|| Status::not_found("Default contest not found"))?)
    }
}

#[tonic::async_trait]
impl Contest for ContestService {
    async fn auth_user(
        &self,
        _request: Request<AuthUserRequest>,
    ) -> Result<Response<AuthUserResponse>, Status> {
        let auth_user = _request.into_inner();
        let auth_username = auth_user.name.clone();
        let auth_password = auth_user.passw.clone();
        Ok(self
            .get_contests_collection()
            .find_one(
                doc! {
                    "users": doc! {
                        "username": auth_username,
                        "password": auth_password
                    }
                },
                None,
            )
            .await
            .map_err(|x| Status::internal(format!("{}", x)))? // TODO fix with error conversion
            .map_or(
                Response::new(AuthUserResponse {
                    error: Some("Incorrect username or password".to_string()),
                    username: None,
                    fullname: None,
                    token: None,
                }),
                |_| {
                    Response::new(AuthUserResponse {
                        username: Some(auth_user.name),
                        fullname: None,
                        error: None,
                        token: None,
                    })
                },
            ))
    }
    async fn get_contest(
        &self,
        _request: Request<GetContestRequest>,
    ) -> Result<Response<GetContestResponse>, Status> {
        todo!();
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
        todo!();
    }
    async fn get_question_list(
        &self,
        _request: Request<GetQuestionListRequest>,
    ) -> Result<Response<GetQuestionListResponse>, Status> {
        todo!();
    }
    async fn set_user(
        &self,
        request: Request<SetUserRequest>,
    ) -> Result<Response<SetUserResponse>, Status> {
        let contests = self.get_contests_collection();
        let default_contest = self.get_default_contest_db().await?;

        let default_contest_id = default_contest
            .get_object_id("_id")
            .map_err(|x| Status::internal(format!("{}", x)))?;

        let users = default_contest
            .get_array("users")
            .map_err(|x| Status::internal(format!("{}", x)))?; // TODO fix with filter
        let new_user = request.into_inner();

        for user in users {
            let user = user
                .as_document()
                .ok_or_else(|| Status::internal("Could not convert to document"))?;
            if user
                .get_str("username")
                .map_err(|x| Status::internal(format!("{}", x)))?
                == new_user.name
            {
                contests
                    .update_one(
                        doc! {
                            "_id": default_contest_id,
                            "users.username": new_user.name
                        },
                        doc! {
                            "$set": doc! {
                                "users.$.password": new_user.passw
                            }
                        },
                        None,
                    )
                    .await
                    .map_err(|x| Status::internal(format!("{}", x)))?;
                return Ok(Response::new(SetUserResponse {
                    code: set_user_response::Code::Update as i32,
                }));
            }
        }

        contests
            .update_one(
                doc! {
                    "_id": default_contest_id
                },
                doc! {
                    "$push": doc! {
                        "users": doc! {
                            "username": new_user.name,
                            "password": new_user.passw
                        }
                    }
                },
                None,
            )
            .await
            .map_err(|x| Status::internal(format!("{}", x)))?;

        Ok(Response::new(SetUserResponse {
            code: set_user_response::Code::Add as i32,
        }))
    }
    async fn set_contest(
        &self,
        _request: Request<SetContestRequest>,
    ) -> Result<Response<SetContestResponse>, Status> {
        todo!();
    }
    async fn set_problem(
        &self,
        _request: Request<SetProblemRequest>,
    ) -> Result<Response<SetProblemResponse>, Status> {
        todo!();
    }
    async fn add_question(
        &self,
        _request: Request<AddQuestionRequest>,
    ) -> Result<Response<AddQuestionResponse>, Status> {
        todo!();
    }
    async fn add_announcement(
        &self,
        _request: Request<AddAnnouncementRequest>,
    ) -> Result<Response<AddAnnouncementResponse>, Status> {
        todo!();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = get_local_address(Service::CONTEST).parse()?;
    let greeter = ContestService::new().await?;

    println!("Starting contest server");
    Server::builder()
        .add_service(ContestServer::new(greeter))
        .serve(addr)
        .await?;
    Ok(())
}
