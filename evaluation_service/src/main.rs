use protos::service::evaluation::{evaluation_server::*, *};
use protos::utils::*;
use tonic::{transport::*, Request, Response, Status};

#[derive(Debug)]
pub struct EvaluationService {}

#[tonic::async_trait]
impl Evaluation for EvaluationService {
    async fn get_testcase_io_files(
        &self,
        _request: Request<GetTestcaseIoFilesRequest>,
    ) -> Result<Response<GetTestcaseIoFilesResponse>, Status> {
        todo!()
    }
    async fn get_problem_scoring(
        &self,
        _request: Request<GetProblemScoringRequest>,
    ) -> Result<Response<GetProblemScoringResponse>, Status> {
        todo!()
    }
    async fn get_contest_user_scoring(
        &self,
        _request: Request<GetContestUserScoringRequest>,
    ) -> Result<Response<GetContestUserScoringResponse>, Status> {
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = get_local_address(Service::EVALUATION).parse()?;
    let evaluation_service = EvaluationService {};

    println!("Starting evaluation server");
    Server::builder()
        .add_service(EvaluationServer::new(evaluation_service))
        .serve(addr)
        .await?;
    Ok(())
}
