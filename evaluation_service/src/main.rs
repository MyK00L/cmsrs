use std::io;
use std::path::PathBuf;

use protos::service::evaluation::{evaluation_server::*, *};
use protos::utils::*;
use tonic::{transport::*, Request, Response, Status};
use utils::storage::FsStorageHelper;

const ROOT_PATH: &str = "/evaluation_files";
const SERIALIZED_EXTENSION: &str = "ser";
const USER_SCORING_FILE_NAME: &str = "user_scoring";
const PROBLEMS_FOLDER_NAME: &str = "problems";
const TESTCASES_FOLDER_NAME: &str = "testcases";
const EVALUATION_FILES_FOLDER_NAME: &str = "files";
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

fn not_found_error<T>(e: T) -> Status
where
    T: core::fmt::Debug + core::fmt::Display,
{
    Status::not_found(format!("{:?}", e))
}

fn not_found_io_error<T>(e: T) -> io::Error
where
    T: core::fmt::Debug + core::fmt::Display,
{
    io::Error::new(io::ErrorKind::NotFound, format!("{:?}", e))
}

#[derive(Debug)]
pub struct EvaluationService {
    storage: FsStorageHelper,
}

impl EvaluationService {
    fn get_problem_folder_from_id(&self, problem_id: u64) -> io::Result<PathBuf> {
        self.storage
            .search_item(None, PROBLEMS_FOLDER_NAME, None)
            .and_then(|op| op.ok_or_else(|| not_found_io_error("Problems folder not found")))
            .and_then(|path| {
                self.storage
                    .search_item(Some(&path), &problem_id.to_string(), None)
            })
            .and_then(|op| {
                op.ok_or_else(|| {
                    not_found_io_error(&format!("Problem not found [id: {}]", problem_id))
                })
            })
    }

    fn get_evaluation_file_resource_name(&self, eval_type: evaluation_file::Type) -> &str {
        match eval_type {
            evaluation_file::Type::Checker => "checker",
            evaluation_file::Type::Interactor => "interactor",
        }
    }
}

#[tonic::async_trait]
impl Evaluation for EvaluationService {
    async fn get_user_scoring(
        &self,
        _request: Request<GetUserScoringRequest>,
    ) -> Result<Response<GetUserScoringResponse>, Status> {
        self.storage
            .search_item(None, USER_SCORING_FILE_NAME, Some(SERIALIZED_EXTENSION))
            .map_err(not_found_error)
            .and_then(|op| op.ok_or_else(|| not_found_error("User scoring method not found")))
            .and_then(|path| {
                self.storage
                    .read_file_object(&path)
                    .map_err(|err| internal_error(err.as_ref()))
            })
            .map(|user| Response::new(GetUserScoringResponse { method: user }))
    }

    async fn get_problem(
        &self,
        request: Request<GetProblemRequest>,
    ) -> Result<Response<GetProblemResponse>, Status> {
        let request = request.into_inner();
        self.get_problem_folder_from_id(request.problem_id)
            .map_err(internal_error)
            .and_then(|path| {
                self.storage
                    .search_item(
                        Some(&path),
                        PROBLEM_METADATA_FILE_NAME,
                        Some(SERIALIZED_EXTENSION),
                    )
                    .map_err(not_found_error)
            })
            .and_then(|op| {
                op.ok_or_else(|| {
                    not_found_error(format!(
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

    async fn get_contest(
        &self,
        _request: Request<GetContestRequest>,
    ) -> Result<Response<GetContestResponse>, Status> {
        let user_scoring_method = self
            .get_user_scoring(Request::new(GetUserScoringRequest {}))
            .await?
            .into_inner()
            .method;
        let mut problems: Vec<Problem> = vec![];
        for entry in self.storage.iterate_folder(PROBLEMS_FOLDER_NAME, None)? {
            let problem_path = self
                .storage
                .search_item(
                    Some(&entry?.path()),
                    PROBLEM_METADATA_FILE_NAME,
                    Some(SERIALIZED_EXTENSION),
                )?
                .ok_or_else(|| not_found_error("Problem metadata not found"))?;
            let p: Problem = self
                .storage
                .read_file_object(&problem_path)
                .map_err(|err| internal_error(err.as_ref()))?;
            problems.push(p);
        }
        Ok(Response::new(GetContestResponse {
            info: Contest {
                problems,
                user_scoring_method,
            },
        }))
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
            .map_err(not_found_error)?;

        // Save problems
        let problems_path = self.storage.add_folder(PROBLEMS_FOLDER_NAME, None)?;
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

            // Create folder for evaluation files
            self.storage
                .add_folder(EVALUATION_FILES_FOLDER_NAME, Some(&p_path))?;
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
        let problem_path = self.get_problem_folder_from_id(problem_id)?;

        // Get testcases folder
        let testcases_path = self
            .storage
            .search_item(Some(&problem_path), TESTCASES_FOLDER_NAME, None)?
            .ok_or_else(|| {
                not_found_error(format!(
                    "Testcases folder not found [problem id: {}]",
                    problem_id
                ))
            })?;

        // Get testcase folder
        let tc_path = self
            .storage
            .search_item(Some(&testcases_path), &testcase_id.to_string(), None)?
            .ok_or_else(|| {
                not_found_error(format!(
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
        request: Request<GetProblemTestcasesRequest>,
    ) -> Result<Response<GetProblemTestcasesResponse>, Status> {
        let request = request.into_inner();
        let problem_id = request.problem_id;
        let mut testcases: Vec<Testcase> = vec![];

        for entry in self
            .get_problem_folder_from_id(problem_id)
            .and_then(|path| {
                self.storage
                    .iterate_folder(TESTCASES_FOLDER_NAME, Some(&path))
            })?
        {
            let testcase_path = entry?.path();
            let testcase_id = testcase_path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .parse::<u64>()
                .map_err(internal_error)?;

            // Get input and output files (if present)
            let input_path = self.storage.search_item(
                Some(&testcase_path),
                INPUT_FILE_NAME,
                Some(IO_EXTENSION),
            )?;
            let input_bytes = match input_path {
                Some(ip) => Some(self.storage.read_file(&ip)?),
                None => None,
            };
            let output_path = self.storage.search_item(
                Some(&testcase_path),
                OUTPUT_FILE_NAME,
                Some(IO_EXTENSION),
            )?;
            let output_bytes = match output_path {
                Some(op) => Some(self.storage.read_file(&op)?),
                None => None,
            };

            testcases.push(Testcase {
                id: testcase_id,
                input: input_bytes,
                output: output_bytes,
            });
        }

        Ok(Response::new(GetProblemTestcasesResponse { testcases }))
    }

    async fn set_testcase(
        &self,
        request: Request<SetTestcaseRequest>,
    ) -> Result<Response<SetTestcaseResponse>, Status> {
        let request = request.into_inner();
        let problem_id = request.problem_id;
        let subtask_id = request.subtask_id;

        // Get testcase's problem path
        let problem_path = self.get_problem_folder_from_id(problem_id)?;

        // Get testcases folder
        let testcases_path = self
            .storage
            .search_item(Some(&problem_path), TESTCASES_FOLDER_NAME, None)
            .map_err(not_found_io_error)
            .and_then(|op| {
                op.ok_or_else(|| {
                    not_found_io_error(format!(
                        "Testcases folder not found [problem id: {}]",
                        problem_id
                    ))
                })
            })?;

        match request.command.unwrap() {
            set_testcase_request::Command::AddTestcase(tc) => {
                // Check and add testcase to problem metadata
                let mut problem_metadata: Problem = self
                    .storage
                    .search_item(
                        Some(&problem_path),
                        PROBLEM_METADATA_FILE_NAME,
                        Some(SERIALIZED_EXTENSION),
                    )?
                    .ok_or_else(|| {
                        not_found_error(format!(
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
                        not_found_error(format!("Subtask not found [id: {}]", subtask_id))
                    })?;
                subtask.testcases_id.push(tc.id);

                self.storage
                    .save_file_object(
                        Some(&problem_path),
                        PROBLEM_METADATA_FILE_NAME,
                        SERIALIZED_EXTENSION,
                        problem_metadata,
                    )
                    .map_err(internal_error)?;

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
            set_testcase_request::Command::UpdateTestcase(tc) => {
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
                    return Err(not_found_error(format!(
                        "Testcase not found [id: {}]",
                        tc.id
                    )));
                }
            }
            set_testcase_request::Command::DeleteTestcaseId(tc_id) => {
                // Delete from problem metadata
                let mut problem_metadata: Problem = self
                    .storage
                    .search_item(
                        Some(&problem_path),
                        PROBLEM_METADATA_FILE_NAME,
                        Some(SERIALIZED_EXTENSION),
                    )?
                    .ok_or_else(|| {
                        not_found_error(format!(
                            "Testcases folder not found [problem id: {}]",
                            problem_id
                        ))
                    })
                    .and_then(|path| {
                        self.storage.read_file_object(&path).map_err(internal_error)
                    })?;

                let subtask = problem_metadata
                    .subtasks
                    .iter_mut()
                    .find(|subtask| subtask.id == subtask_id)
                    .ok_or_else(|| {
                        not_found_error(format!("Subtask not found [id: {}]", subtask_id))
                    })?;
                let index = subtask
                    .testcases_id
                    .iter()
                    .position(|&id| id == tc_id)
                    .ok_or_else(|| {
                        not_found_error(format!("Testcase not found [id: {}]", tc_id))
                    })?;
                subtask.testcases_id.remove(index);

                self.storage
                    .save_file_object(
                        Some(&problem_path),
                        PROBLEM_METADATA_FILE_NAME,
                        SERIALIZED_EXTENSION,
                        problem_metadata,
                    )
                    .map_err(internal_error)?;

                // Delete from storage
                let tc_path =
                    self.storage
                        .search_item(Some(&testcases_path), &tc_id.to_string(), None)?;
                if let Some(tc_path) = tc_path {
                    self.storage.delete_item(&tc_path)?;
                } else {
                    return Err(not_found_error(format!(
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
        request: Request<GetProblemEvaluationFileRequest>,
    ) -> Result<Response<GetProblemEvaluationFileResponse>, Status> {
        let request = request.into_inner();
        let problem_id = request.problem_id;

        let problem_path = self.get_problem_folder_from_id(problem_id)?;
        let file_type = evaluation_file::Type::from_i32(request.r#type).ok_or_else(|| {
            internal_error(format!(
                "Cannot convert {} to evaluation file type",
                request.r#type
            ))
        })?;

        let eval_path = self
            .storage
            .search_item(Some(&problem_path), EVALUATION_FILES_FOLDER_NAME, None)?
            .ok_or_else(|| not_found_io_error("Evaluation files folder not found"))?;
        self.storage
            .search_item(
                Some(&eval_path),
                self.get_evaluation_file_resource_name(file_type),
                Some(SERIALIZED_EXTENSION),
            )?
            .ok_or_else(|| not_found_error("Evaluation file not found"))
            .and_then(|path| self.storage.read_file_object(&path).map_err(internal_error))
            .map(|file| Response::new(GetProblemEvaluationFileResponse { file }))
    }

    async fn set_problem_evaluation_file(
        &self,
        request: Request<SetProblemEvaluationFileRequest>,
    ) -> Result<Response<SetProblemEvaluationFileResponse>, Status> {
        let request = request.into_inner();
        let problem_id = request.problem_id;

        let problem_path = self.get_problem_folder_from_id(problem_id)?;

        match request.command.unwrap() {
            set_problem_evaluation_file_request::Command::AddEvaluationFile(ef) => {
                let file_type = evaluation_file::Type::from_i32(ef.r#type).ok_or_else(|| {
                    internal_error(format!(
                        "Cannot convert {} to evaluation file type",
                        ef.r#type
                    ))
                })?;
                let eval_path = self
                    .storage
                    .add_folder(EVALUATION_FILES_FOLDER_NAME, Some(&problem_path))?;
                self.storage
                    .save_file_object(
                        Some(&eval_path),
                        self.get_evaluation_file_resource_name(file_type),
                        SERIALIZED_EXTENSION,
                        ef,
                    )
                    .map_err(internal_error)?;
            }
            set_problem_evaluation_file_request::Command::UpdateEvaluationFile(ef) => {
                let file_type = evaluation_file::Type::from_i32(ef.r#type).ok_or_else(|| {
                    internal_error(format!(
                        "Cannot convert {} to evaluation file type",
                        ef.r#type
                    ))
                })?;
                let eval_path = self
                    .storage
                    .search_item(Some(&problem_path), EVALUATION_FILES_FOLDER_NAME, None)?
                    .ok_or_else(|| not_found_io_error("Evaluation files folder not found"))?;
                self.storage
                    .save_file_object(
                        Some(&eval_path),
                        self.get_evaluation_file_resource_name(file_type),
                        SERIALIZED_EXTENSION,
                        ef,
                    )
                    .map_err(internal_error)?;
            }
        }
        Ok(Response::new(SetProblemEvaluationFileResponse {}))
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
