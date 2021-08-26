use protos::service::evaluation::{evaluation_server::*, *};
use protos::utils::*;
use tonic::{transport::*, Request, Response, Status};

#[derive(Debug)]
pub struct EvaluationService {}

#[tonic::async_trait]
impl Evaluation for EvaluationService {
    async fn get_problem_scoring_info(
        &self,
        _request: Request<GetProblemScoringInfoRequest>,
    ) -> Result<Response<GetProblemScoringInfoResponse>, Status> {
        todo!()
    }
    async fn get_problem_evaluation_info(
        &self,
        _request: Request<GetProblemEvaluationInfoRequest>,
    ) -> Result<Response<GetProblemEvaluationInfoResponse>, Status> {
        todo!()
    }
    async fn get_problem_testcases(
        &self,
        _request: Request<GetProblemTestcasesRequest>,
    ) -> Result<Response<GetProblemTestcasesResponse>, Status> {
        todo!()
    }
    async fn get_problem_evaluation_file(
        &self,
        _request: Request<GetProblemEvaluationFileRequest>,
    ) -> Result<Response<GetProblemEvaluationFileResponse>, Status> {
        todo!()
    }
    async fn get_user_scoring_method(
        &self,
        _request: Request<GetUserScoringMethodRequest>,
    ) -> Result<Response<GetUserScoringMethodResponse>, Status> {
        todo!()
    }
    async fn set_problem_info(
        &self,
        _request: Request<SetProblemInfoRequest>,
    ) -> Result<Response<SetProblemInfoResponse>, Status> {
        todo!()
    }
    async fn set_user_scoring_method(
        &self,
        _request: Request<SetUserScoringMethodRequest>,
    ) -> Result<Response<SetUserScoringMethodResponse>, Status> {
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
