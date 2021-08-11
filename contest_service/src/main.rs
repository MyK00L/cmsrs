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
}

#[tonic::async_trait]
impl Contest for ContestService {
    async fn auth_user(
        &self,
        request: Request<AuthUserRequest>,
    ) -> Result<Response<AuthUserResponse>, Status> {
        todo!();
    }
    async fn get_contest(
        &self,
        request: Request<GetContestRequest>,
    ) -> Result<Response<GetContestResponse>, Status> {
        todo!();
    }
    async fn get_problem(
        &self,
        request: Request<GetProblemRequest>,
    ) -> Result<Response<GetProblemResponse>, Status> {
        todo!();
    }
    async fn get_announcement_list(
        &self,
        request: Request<GetAnnouncementListRequest>,
    ) -> Result<Response<GetAnnouncementListResponse>, Status> {
        todo!();
    }
    async fn get_question_list(
        &self,
        request: Request<GetQuestionListRequest>,
    ) -> Result<Response<GetQuestionListResponse>, Status> {
        todo!();
    }
    async fn set_user(
        &self,
        request: Request<SetUserRequest>,
    ) -> Result<Response<SetUserResponse>, Status> {
        let db = self.client.database("contestdb");
        let contests = db.collection::<Document>("contests");

        let default_contest = contests
            .find_one(None, None)
            .await
            .unwrap() // TODO fix with error conversion
            .ok_or(Status::not_found("Default contest not found"))?;

        let default_contest_id = default_contest.get_object_id("_id").unwrap();

        let users = default_contest.get_array("users").unwrap(); // TODO fix with filter
        let new_user = request.into_inner();

        for user in users {
            let user = user.as_document().unwrap();
            if user.get_str("username").unwrap() == new_user.name {
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
                    .unwrap();
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
            .unwrap();

        Ok(Response::new(SetUserResponse {
            code: set_user_response::Code::Add as i32,
        }))
    }
    async fn set_contest(
        &self,
        request: Request<SetContestRequest>,
    ) -> Result<Response<SetContestResponse>, Status> {
        todo!();
    }
    async fn set_problem(
        &self,
        request: Request<SetProblemRequest>,
    ) -> Result<Response<SetProblemResponse>, Status> {
        todo!();
    }
    async fn add_question(
        &self,
        request: Request<AddQuestionRequest>,
    ) -> Result<Response<AddQuestionResponse>, Status> {
        todo!();
    }
    async fn add_announcement(
        &self,
        request: Request<AddAnnouncementRequest>,
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
