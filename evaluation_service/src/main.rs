use protos::service::evaluation::{evaluation_server::*, *};
use protos::utils::*;
use tonic::{transport::*, Request, Response, Status};
use utils::storage::FsStorageHelper;

const ROOT_PATH: &str = "/evaluation_files";
const SERIALIZED_EXTENSION: &str = ".ser";
const USER_SCORING_FILE_NAME: &str = "user_scoring";
const PROBLEMS_FOLDER_NAME: &str = "problems";
const PROBLEM_METADATA_FILE_NAME: &str = "metadata";

fn internal_error<T>(e: T) -> Status
where
    T: core::fmt::Debug + core::fmt::Display,
{
    Status::internal(format!("{:?}", e))
}

#[derive(Debug)]
pub struct EvaluationService {
    storage: FsStorageHelper,
}

#[tonic::async_trait]
impl Evaluation for EvaluationService {
    async fn get_user_scoring(
        &self,
        _request: Request<GetUserScoringRequest>,
    ) -> Result<Response<GetUserScoringResponse>, Status> {
        self.storage
            .search_item(None, USER_SCORING_FILE_NAME, Some(SERIALIZED_EXTENSION))
            .map_err(internal_error)
            .and_then(|op| op.ok_or_else(|| Status::not_found("User scoring method not found")))
            .and_then(|path| {
                self.storage
                    .read_file_object(&path)
                    .map_err(|err| internal_error(err.as_ref()))
            })
            .map(|user| Response::new(GetUserScoringResponse { info: user }))
    }

    async fn get_problem(
        &self,
        request: Request<GetProblemRequest>,
    ) -> Result<Response<GetProblemResponse>, Status> {
        let request = request.into_inner();
        self.storage
            .search_item(None, PROBLEMS_FOLDER_NAME, None)
            .map_err(internal_error)
            .and_then(|op| op.ok_or_else(|| Status::not_found("Problems folder not found")))
            .and_then(|path| {
                self.storage
                    .search_item(Some(&path), &request.problem_id.to_string(), None)
                    .map_err(internal_error)
            })
            .and_then(|op| {
                op.ok_or_else(|| {
                    Status::not_found(format!("Problem not found [id: {}]", request.problem_id))
                })
            })
            .and_then(|path| {
                self.storage
                    .search_item(
                        Some(&path),
                        PROBLEM_METADATA_FILE_NAME,
                        Some(SERIALIZED_EXTENSION),
                    )
                    .map_err(internal_error)
            })
            .and_then(|op| {
                op.ok_or_else(|| {
                    Status::not_found(format!(
                        "Problem metadata not found [id: {}]",
                        request.problem_id
                    ))
                })
            })
            .and_then(|path| {
                self.storage
                    .read_file_object(&path)
                    .map_err(|err| internal_error(err.as_ref()))
            })
            .map(|prob| Response::new(GetProblemResponse { info: prob }))
    }

    async fn set_contest(
        &self,
        request: Request<SetContestRequest>,
    ) -> Result<Response<SetContestResponse>, Status> {
        let request = request.into_inner();
        let user_scoring_method = request.info.user_scoring_method;
        let problems = request.info.problems;

        // Save user scoring method
        self.storage
            .save_file_object(
                None,
                USER_SCORING_FILE_NAME,
                SERIALIZED_EXTENSION,
                user_scoring_method,
            )
            .map_err(internal_error)?;

        // Save problems
        let problems_path = self
            .storage
            .search_item(None, PROBLEMS_FOLDER_NAME, None)?
            .ok_or_else(|| Status::not_found("Problems folder not found"))?;
        for p in problems {
            let p_path = self
                .storage
                .add_folder(&p.id.to_string(), Some(&problems_path))
                .map_err(internal_error)?;
            self.storage
                .save_file_object(
                    Some(&p_path),
                    PROBLEM_METADATA_FILE_NAME,
                    SERIALIZED_EXTENSION,
                    p,
                )
                .map_err(internal_error)?;
        }
        Ok(Response::new(SetContestResponse {}))
    }

    async fn get_testcase(
        &self,
        _request: Request<GetTestcaseRequest>,
    ) -> Result<Response<GetTestcaseResponse>, Status> {
        todo!()
    }

    async fn get_problem_testcases(
        &self,
        _request: Request<GetProblemTestcasesRequest>,
    ) -> Result<Response<GetProblemTestcasesResponse>, Status> {
        todo!()
    }

    async fn set_testcase(
        &self,
        _request: Request<SetTestcaseRequest>,
    ) -> Result<Response<SetTestcaseResponse>, Status> {
        todo!()
    }

    async fn get_problem_evaluation_file(
        &self,
        _request: Request<GetProblemEvaluationFileRequest>,
    ) -> Result<Response<GetProblemEvaluationFileResponse>, Status> {
        todo!()
    }

    async fn set_problem_evaluation_file(
        &self,
        _request: Request<SetProblemEvaluationFileRequest>,
    ) -> Result<Response<SetProblemEvaluationFileResponse>, Status> {
        todo!()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = get_local_address(Service::EVALUATION).parse()?;
    let evaluation_service = EvaluationService {
        storage: FsStorageHelper::new(std::path::Path::new(ROOT_PATH))?,
    };

    println!("Starting evaluation server");
    Server::builder()
        .add_service(EvaluationServer::new(evaluation_service))
        .serve(addr)
        .await?;
    Ok(())
}
