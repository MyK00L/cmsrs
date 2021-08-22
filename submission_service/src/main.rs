
use mongodb::{
    bson::{doc, Document},
    options::{ClientOptions, UpdateOptions},
    Client,
};
use protos::service::submission::{submission_server::*, *};
use protos::utils::*;
use tonic::{transport::*, Request, Response, Status};

#[cfg(test)]
mod tests;

// TODO: remove credentials to connect to db.
const CONNECTION_STRING: &str = "mongodb://root:example@submission_service_db:27017/";

fn internal_error<T>(e: T) -> Status
where
    T: std::fmt::Debug,
{
    Status::internal(format!("{:?}", e))
}

#[derive(Debug)]
pub struct SubmissionService {
    db_client: Client,
}

impl SubmissionService {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            db_client: Client::with_options(ClientOptions::parse(CONNECTION_STRING).await?)?,
        })
    }

    fn get_collection(&self) -> mongodb::Collection<Document> {
        self.db_client
            .database("submissionsdb")
            .collection::<Document>("submissions")
    }
}

#[tonic::async_trait]
impl Submission for SubmissionService {
    // TODO implement the RPCs here (signatures in submission.proto file)
    async fn evaluate_submission(
        &self, 
        _request: Request<EvaluateSubmissionRequest>
    ) -> Result<Response<EvaluateSubmissionResponse>, Status> { 
        todo!() 
    }

    /*
    get_submission_list --> based on the author, we keep an index on authorUsername
    */
    async fn get_submission_list(
        &self,
        request: Request<GetSubmissionListRequest>
    ) -> Result<Response<GetSubmissionListResponse>, Status> { 
        todo!()
        /*let list_request = request.into_inner();
        let limit = list_request.limit.clone();
        let author_username = list_request.author_username.clone();
        let problem_id = list_request.problem_id.clone();



        Ok(self
            .get_collection()
            .find(
                
            ))
        */
    }

    fn get_submission_details(
        &self,
        _request: Request<GetSubmissionDetailsRequest>
    ) -> Result<Response<GetSubmissionDetailsResponse>, Status> { 
        todo!() 
    }

/*
    async fn auth_user(
        &self,
        request: Request<AuthUserRequest>,
    ) -> Result<Response<AuthUserResponse>, Status> {
    
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
            .map(protos::user::Message::from)
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
            .map(protos::user::Message::from)
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
            .map_err(|err| Status::invalid_argument(format!("{:?}", err)))?;

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
    async fn add_message(
        &self,
        request: Request<AddMessageRequest>,
    ) -> Result<Response<AddMessageResponse>, Status> {
        let message = mappings::chat::Message::from(request.into_inner());
        // TODO should we notify someone here?
        let doc = Document::from(message);
        if message.is_question() {
            self.get_questions_collection()
        } else {
            self.get_announcements_collection()
        }
        .insert_one(doc, None)
        .await
        .map_err(internal_error)?;
        Ok(Response::new(AddMessageResponse {}))
    }
    */
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = get_local_address(Service::CONTEST).parse()?;
    let submission_service = SubmissionService::new().await?;

    println!("Starting submission server");
    Server::builder()
        .add_service(SubmissionServer::new(submission_service))
        .serve(addr)
        .await?;
    Ok(())
}