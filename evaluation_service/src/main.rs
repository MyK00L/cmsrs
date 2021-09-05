use protos::service::evaluation::{evaluation_server::*, set_testcase_request::Command, *};
use protos::utils::*;
use tonic::{transport::*, Request, Response, Status};
use utils::storage::FsStorageHelper;

const ROOT_PATH: &str = "/evaluation_files";
const SERIALIZED_EXTENSION: &str = ".ser";
const USER_SCORING_FILE_NAME: &str = "user_scoring";
const PROBLEMS_FOLDER_NAME: &str = "problems";
const TESTCASES_FOLDER_NAME: &str = "testcases";
const INPUT_FILE_NAME: &str = "input";
const OUTPUT_FILE_NAME: &str = "output";
const IO_EXTENSION: &str = "txt";
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
        for p in problems.iter() {
            // Save problem metadata
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

            // Create folders for testcases
            let testcases_path = self
                .storage
                .add_folder(TESTCASES_FOLDER_NAME, Some(&p_path))?;
            for tc_id in p.subtasks.iter().flat_map(|sub| sub.testcases_id.iter()) {
                self.storage
                    .add_folder(&tc_id.to_string(), Some(&testcases_path))?;
            }
        }
        Ok(Response::new(SetContestResponse {}))
    }

    async fn get_testcase(
        &self,
        request: Request<GetTestcaseRequest>,
    ) -> Result<Response<GetTestcaseResponse>, Status> {
        let request = request.into_inner();
        let problem_id = request.problem_id;
        let testcase_id = request.testcase_id;

        // Get testcase's problem path
        let problem_path = self
            .storage
            .search_item(None, PROBLEMS_FOLDER_NAME, None)
            .map_err(internal_error)
            .and_then(|op| op.ok_or_else(|| Status::not_found("Problems folder not found")))
            .and_then(|path| {
                self.storage
                    .search_item(Some(&path), &problem_id.to_string(), None)
                    .map_err(internal_error)
            })
            .and_then(|op| {
                op.ok_or_else(|| {
                    Status::not_found(format!("Problem not found [id: {}]", problem_id))
                })
            })?;

        // Get testcases folder
        let testcases_path = self
            .storage
            .search_item(Some(&problem_path), TESTCASES_FOLDER_NAME, None)?
            .ok_or_else(|| {
                Status::not_found(format!(
                    "Testcases folder not found [problem id: {}]",
                    problem_id
                ))
            })?;

        // Get testcase folder
        let tc_path = self
            .storage
            .search_item(Some(&testcases_path), &testcase_id.to_string(), None)?
            .ok_or_else(|| {
                Status::not_found(format!(
                    "Testcase folder not found [problem id: {}, id: {}]",
                    problem_id, testcase_id
                ))
            })?;

        // Get input and output files (if present)
        let input_path =
            self.storage
                .search_item(Some(&tc_path), INPUT_FILE_NAME, Some(IO_EXTENSION))?;
        let input_bytes = match input_path {
            Some(ip) => Some(self.storage.read_file(&ip)?),
            None => None,
        };
        let output_path =
            self.storage
                .search_item(Some(&tc_path), OUTPUT_FILE_NAME, Some(IO_EXTENSION))?;
        let output_bytes = match output_path {
            Some(op) => Some(self.storage.read_file(&op)?),
            None => None,
        };

        Ok(Response::new(GetTestcaseResponse {
            testcase: Testcase {
                id: testcase_id,
                input: input_bytes,
                output: output_bytes,
            },
        }))
    }

    async fn get_problem_testcases(
        &self,
        _request: Request<GetProblemTestcasesRequest>,
    ) -> Result<Response<GetProblemTestcasesResponse>, Status> {
        todo!()
    }

    async fn set_testcase(
        &self,
        request: Request<SetTestcaseRequest>,
    ) -> Result<Response<SetTestcaseResponse>, Status> {
        let request = request.into_inner();
        let problem_id = request.problem_id;
        let subtask_id = request.subtask_id;

        // Get testcase's problem path
        let problem_path = self
            .storage
            .search_item(None, PROBLEMS_FOLDER_NAME, None)
            .map_err(internal_error)
            .and_then(|op| op.ok_or_else(|| Status::not_found("Problems folder not found")))
            .and_then(|path| {
                self.storage
                    .search_item(Some(&path), &problem_id.to_string(), None)
                    .map_err(internal_error)
            })
            .and_then(|op| {
                op.ok_or_else(|| {
                    Status::not_found(format!("Problem not found [id: {}]", problem_id))
                })
            })?;

        // Get testcases folder
        let testcases_path = self
            .storage
            .search_item(Some(&problem_path), TESTCASES_FOLDER_NAME, None)
            .map_err(internal_error)
            .and_then(|op| {
                op.ok_or_else(|| {
                    Status::not_found(format!(
                        "Testcases folder not found [problem id: {}]",
                        problem_id
                    ))
                })
            })?;

        match request.command.unwrap() {
            Command::AddTestcase(tc) => {
                // Check and add testcase to problem metadata
                let mut problem_metadata: Problem = self
                    .storage
                    .search_item(
                        Some(&problem_path),
                        PROBLEM_METADATA_FILE_NAME,
                        Some(SERIALIZED_EXTENSION),
                    )?
                    .ok_or_else(|| {
                        Status::not_found(format!(
                            "Testcases folder not found [problem id: {}]",
                            problem_id
                        ))
                    })
                    .and_then(|path| {
                        self.storage.read_file_object(&path).map_err(internal_error)
                    })?;

                if problem_metadata
                    .subtasks
                    .iter()
                    .flat_map(|subtask| subtask.testcases_id.iter())
                    .any(|&tid| tid == tc.id)
                {
                    return Err(Status::already_exists(format!(
                        "Testcase already exists [problem id: {}, id: {}]",
                        problem_id, tc.id
                    )));
                }

                let subtask = problem_metadata
                    .subtasks
                    .iter_mut()
                    .find(|subtask| subtask.id == subtask_id)
                    .ok_or_else(|| {
                        Status::not_found(format!("Subtask not found [id: {}]", subtask_id))
                    })?;
                subtask.testcases_id.push(tc.id);

                // Save testcase files into storage (only if present)
                let tc_path = self
                    .storage
                    .add_folder(&tc.id.to_string(), Some(&testcases_path))?;

                if tc.input.is_some() {
                    self.storage.save_file(
                        Some(&tc_path),
                        INPUT_FILE_NAME,
                        IO_EXTENSION,
                        tc.input(),
                    )?;
                }
                if tc.output.is_some() {
                    self.storage.save_file(
                        Some(&tc_path),
                        OUTPUT_FILE_NAME,
                        IO_EXTENSION,
                        tc.output(),
                    )?;
                }
            }
            Command::UpdateTestcase(tc) => {
                let tc_path =
                    self.storage
                        .search_item(Some(&testcases_path), &tc.id.to_string(), None)?;
                if let Some(tc_path) = tc_path {
                    self.storage.save_file(
                        Some(&tc_path),
                        INPUT_FILE_NAME,
                        IO_EXTENSION,
                        tc.input(),
                    )?;
                    self.storage.save_file(
                        Some(&tc_path),
                        OUTPUT_FILE_NAME,
                        IO_EXTENSION,
                        tc.output(),
                    )?;
                } else {
                    return Err(Status::not_found(format!(
                        "Testcase not found [id: {}]",
                        tc.id
                    )));
                }
            }
            Command::DeleteTestcaseId(tc_id) => {
                let tc_path =
                    self.storage
                        .search_item(Some(&testcases_path), &tc_id.to_string(), None)?;
                if let Some(tc_path) = tc_path {
                    self.storage.delete_item(&tc_path)?;
                } else {
                    return Err(Status::not_found(format!(
                        "Testcase not found [id: {}]",
                        tc_id
                    )));
                }
            }
        };
        Ok(Response::new(SetTestcaseResponse {}))
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
