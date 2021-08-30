use protos::service::evaluation::{evaluation_server::*, *};
use protos::utils::*;
use tonic::{transport::*, Request, Response, Status};

#[derive(Debug)]
pub struct EvaluationService {}

#[tonic::async_trait]
impl Evaluation for EvaluationService {
    async fn get_user_scoring(
        &self,
        _request: Request<GetUserScoringRequest>,
    ) -> Result<Response<GetUserScoringResponse>, Status> {
        todo!()
    }
    async fn set_user_scoring(
        &self,
        _request: Request<SetUserScoringRequest>,
    ) -> Result<Response<SetUserScoringResponse>, Status> {
        todo!()
    }
    async fn get_problem(
        &self,
        _request: Request<GetProblemRequest>,
    ) -> Result<Response<GetProblemResponse>, Status> {
        todo!()
    }
    async fn set_problem(
        &self,
        _request: Request<SetProblemRequest>,
    ) -> Result<Response<SetProblemResponse>, Status> {
        todo!()
    }
    async fn get_testcase(
        &self,
        _request: Request<GetTestcaseRequest>,
    ) -> Result<Response<GetTestcaseResponse>, Status> {
        todo!()
    }
    async fn set_testcase(
        &self,
        _request: Request<SetTestcaseRequest>,
    ) -> Result<Response<SetTestcaseResponse>, Status> {
        todo!()
    }
    async fn get_evaluation_file(
        &self,
        _request: Request<GetEvaluationFileRequest>,
    ) -> Result<Response<GetEvaluationFileResponse>, Status> {
        todo!()
    }
    async fn set_evaluation_file(
        &self,
        _request: Request<SetEvaluationFileRequest>,
    ) -> Result<Response<SetEvaluationFileResponse>, Status> {
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
