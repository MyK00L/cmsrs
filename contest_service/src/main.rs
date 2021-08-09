use protos::service::contest::{contest_server::*, *};
use protos::utils::*;
use tonic::{transport::*, Request, Response, Status};

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct ContestService {}

impl Default for ContestService {
    fn default() -> Self {
        Self {}
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
    async fn add_user(
        &self,
        request: Request<AddUserRequest>,
    ) -> Result<Response<AddUserResponse>, Status> {
        todo!();
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
    let greeter = ContestService::default();

    println!("Starting contest server");
    Server::builder()
        .add_service(ContestServer::new(greeter))
        .serve(addr)
        .await?;
    Ok(())
}
