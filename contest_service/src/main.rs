#![feature(async_closure)]
use futures::stream::StreamExt;
use std::convert::TryFrom;

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

mod mongo_schema;
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
        let db_client = Client::with_options(ClientOptions::parse(CONNECTION_STRING).await?)?;
        mongo_schema::init_contest_service_db(db_client.database("contestdb")).await?;
        Ok(Self { db_client })
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
            .find_one(doc! { "_id": username }, None)
            .await
            .map_err(internal_error)?
            .map(mappings::user::User::from)
            .filter(|u| {
                if let Ok(r) = u.clone().verify_password(&user.password) {
                    r
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
                |u| {
                    Response::new(AuthUserResponse {
                        response: Some(u.into()),
                    })
                },
            ))
    }
    async fn get_contest_metadata(
        &self,
        _request: Request<GetContestMetadataRequest>,
    ) -> Result<Response<GetContestMetadataResponse>, Status> {
        let metadata = self
            .get_contest_metadata()
            .await
            .map(|contest_metadata_doc| {
                mappings::contest::ContestMetadata::from(contest_metadata_doc).into()
            })?;
        let problems = self
            .get_problems_collection()
            .find(None, None)
            .await
            .map_err(internal_error)?
            .filter_map(async move |x| x.ok())
            .map(mappings::problem::ProblemData::from)
            .map(|x| x.get_problem().into())
            .collect()
            .await;
        Ok(Response::new(GetContestMetadataResponse {
            metadata,
            problems,
        }))
    }
    async fn get_problem_info(
        &self,
        request: Request<GetProblemRequest>,
    ) -> Result<Response<GetProblemInfoResponse>, Status> {
        let problem_id = request.into_inner().problem_id;
        self.get_problems_collection()
            .find_one(doc! {"_id": problem_id as i64}, None)
            .await
            .map_err(internal_error)?
            .map(mappings::problem::ProblemData::from)
            .map(|x| {
                Response::new(GetProblemInfoResponse {
                    info: x.get_problem().into(),
                })
            })
            .ok_or_else(|| Status::not_found("Problem not found"))
    }

    async fn get_problem_statement(
        &self,
        request: Request<GetProblemRequest>,
    ) -> Result<Response<GetProblemStatementResponse>, Status> {
        let problem_id = request.into_inner().problem_id;
        self.get_problems_collection()
            .find_one(doc! {"_id": problem_id as i64}, None)
            .await
            .map_err(internal_error)?
            .map(mappings::problem::ProblemData::from)
            .map(|x| {
                Response::new(GetProblemStatementResponse {
                    statement: x.get_statement(),
                })
            })
            .ok_or_else(|| Status::not_found("Problem not found"))
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
        let user: mappings::user::User = request.into_inner().into();

        self.get_users_collection()
            .update_one(
                doc! { "_id": user.get_username() },
                doc! { "$set": Document::from(user) },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
            .map_err(|err| Status::internal(format!("{}", err)))
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
        let problem_data: mappings::problem::ProblemData = (
            problem_data_from_req.info.into(),
            problem_data_from_req.statement,
        )
            .into();

        let document: Document = problem_data.into();
        self.get_problems_collection()
            .update_one(
                doc! { "_id": document.get_i64("_id").unwrap() },
                doc! { "$set": document },
                UpdateOptions::builder().upsert(true).build(),
            )
            .await
            .map_err(internal_error)
            .map(|_| Response::new(SetProblemResponse {}))
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

    async fn update_problem_info(
        &self,
        request: Request<UpdateProblemInfoRequest>,
    ) -> Result<Response<SetProblemResponse>, Status> {
        let problem_data_from_req = request.into_inner();
        let problem_data: mappings::problem::Problem = problem_data_from_req.info.into();
        self.get_problems_collection()
            .update_one(
                doc! { "_id": problem_data.get_id() },
                doc! { "$set": doc!{"name": problem_data.name, "longName": problem_data.long_name} },
                UpdateOptions::builder().build(),
            )
            .await
            .map_err(internal_error)
            .map(|_| Response::new(SetProblemResponse {}))
    }

    async fn update_problem_statement(
        &self,
        request: Request<UpdateProblemStatementRequest>,
    ) -> Result<Response<SetProblemResponse>, Status> {
        let problem_data_from_req = request.into_inner();
        let problem_statement = problem_data_from_req.statement;
        let problem_id = problem_data_from_req.problem_id;

        self.get_problems_collection()
            .update_one(
                doc! { "_id": problem_id as i64 },
                doc! { "$set": doc!{"statement": mongodb::bson::Binary {
                    subtype: mongodb::bson::spec::BinarySubtype::Generic,
                    bytes: problem_statement,
                }} },
                UpdateOptions::builder().build(),
            )
            .await
            .map_err(internal_error)
            .map(|_| Response::new(SetProblemResponse {}))
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
