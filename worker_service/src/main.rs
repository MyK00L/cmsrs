use failure::{Error, Fail, format_err};
use protos::{
    common::{self, Duration, ProgrammingLanguage, Resources, Score, Source, Timestamp},
    evaluation::{compilation_result, testcase_result::Outcome, CompilationResult, TestcaseResult},
    scoring,
    service::{
        evaluation::{
            evaluation_client::EvaluationClient, evaluation_file, evaluation_server::Evaluation,
            problem, EvaluationFile, GetProblemEvaluationFileRequest, GetProblemRequest,
            GetProblemResponse, GetProblemTestcasesRequest, GetProblemTestcasesResponse,
            GetTestcaseRequest, MockEvaluation, Problem, Testcase,
        },
        worker::{
            worker_server::{Worker, WorkerServer},
            EvaluateSubmissionRequest, EvaluateSubmissionResponse, UpdateSourceRequest,
            UpdateSourceResponse, UpdateTestcaseRequest, UpdateTestcaseResponse,
        },
    },
    utils::{get_local_address, Service},
    worker::{self},
};
use futures::lock::Mutex;
use std::{collections::HashMap, fs::File, iter::Map, path::{Path, PathBuf}, process::ExitStatus, sync::{Arc, MutexGuard}, thread::{sleep, spawn, JoinHandle}};
use tabox::{
    configuration::SandboxConfiguration,
    result::SandboxExecutionResult,
    syscall_filter::{SyscallFilter, SyscallFilterAction},
    SandboxImplementation,
};
use tabox::{result::ResourceUsage, Sandbox};
use tonic::{transport::Server, Code, Request, Response, Status};

#[path = "./configurations.rs"]
mod configurations;
use configurations::*;

fn timestamp_cmp(a: Timestamp, b: Timestamp) -> i64 {
    if a.secs == b.secs {
        (a.nanos as i64) - (b.nanos as i64)
    } else {
        (a.secs as i64) - (b.secs as i64)
    }
}

type ProblemId = u64;
type TestcaseId = u64;

#[derive(Debug)]
struct FileStatus {
    testcases: HashMap<(ProblemId, TestcaseId), Timestamp>,
    checkers: HashMap<(ProblemId, evaluation_file::Type), Timestamp>,
}

impl FileStatus {
    fn new() -> Self {
        FileStatus {
            testcases: HashMap::new(),
            checkers: HashMap::new(),
        }
    }
}

struct EvaluationFileStatus {
    // vectors of id and correspondent timestamp
    testcases: Vec<(ProblemId, TestcaseId, Timestamp)>,
    checkers: Vec<(ProblemId, evaluation_file::Type, Timestamp)>,
}

pub struct WorkerService {
    status: Arc<Mutex<FileStatus>>,
    evaluation_service: EvaluationClient<tonic::transport::Channel>,
}

async fn pull_testcase(
    evaluation_service: &mut EvaluationClient<tonic::transport::Channel>,
    problem_id: ProblemId,
    testcase_id: TestcaseId,
) -> Testcase {
    loop {
        if let Ok(testcase) = evaluation_service
            .get_testcase(Request::new(GetTestcaseRequest {
                problem_id,
                testcase_id,
            }))
            .await
        {
            return testcase.into_inner().testcase;
        }
    }
}

async fn pull_checker(
    evaluation_service: &mut EvaluationClient<tonic::transport::Channel>,
    problem_id: ProblemId,
    checker_type: evaluation_file::Type,
) -> EvaluationFile {
    loop {
        if let Ok(evaluation_file) = evaluation_service
            .get_problem_evaluation_file(Request::new(GetProblemEvaluationFileRequest {
                problem_id,
                r#type: checker_type as i32,
            }))
            .await
        {
            return evaluation_file.into_inner().file;
        }
    }
}

async fn diff_and_update_status(
    evaluation_service: &mut EvaluationClient<tonic::transport::Channel>,
    wrapped_status: &Arc<Mutex<FileStatus>>,
    actual_status: EvaluationFileStatus,
) {
    let mut status = wrapped_status.lock().await;

    for (problem_id, testcase_id, actual_timestamp) in actual_status.testcases {
        // insert the new one and get the older value associated to the key
        let old_timestamp = status
            .testcases
            .insert((problem_id, testcase_id), actual_timestamp.clone());

        if old_timestamp.is_none() || timestamp_cmp(old_timestamp.unwrap(), actual_timestamp) < 0 {
            // pull updated testcase
            let testcase = pull_testcase(evaluation_service, problem_id, testcase_id).await;
            // save testcase
            let testcase_dir = get_testcase_dir_path(problem_id, testcase_id);
            let input_file_path = testcase_dir.join(PathBuf::from("input.txt"));
            let output_file_path = testcase_dir.join(PathBuf::from("output.txt"));

            save_file(
                testcase.input.expect("Testcase input should be present"),
                input_file_path,
            )
            .expect("Unable to save the testcase input");
            save_file(
                testcase.output.expect("Testcase output should be present"),
                output_file_path,
            )
            .expect("Unable to save the testcase output");
        }
    }

    for (problem_id, checker_type, actual_timestamp) in actual_status.checkers {
        // insert the new one and get the older value associated to the key
        let old_timestamp = status
            .checkers
            .insert((problem_id, checker_type), actual_timestamp.clone());

        if old_timestamp.is_none() || timestamp_cmp(old_timestamp.unwrap(), actual_timestamp) < 0 {
            // pull updated checker
            let checker = pull_checker(evaluation_service, problem_id, checker_type).await;
            // compile checker in sandbox
            let config =
                get_checker_compilation_config(problem_id, checker.r#type(), checker.source)
                    .expect("Unable to get checker compilation configuration.");
            let sandbox_res = run_sandbox(config).expect("Failed to run sandbox.");
            if !sandbox_res.status.success() {
                panic!();
            }
            // success is good
        }
    }
}

async fn pull_join_handler_action(
    evaluation_service: EvaluationClient<tonic::transport::Channel>,
    wrapped_status: Arc<Mutex<FileStatus>>
) {
    let mut evaluation_service = evaluation_service.clone();
    sleep(std::time::Duration::new(30, 0));
    loop {
        // sleep 30 secs
        sleep(std::time::Duration::new(30, 0));

        // pull status. TODO actually do the pull
        let actual_status = EvaluationFileStatus {
            testcases: vec![],
            checkers: vec![],
        };

        // diff and update
        diff_and_update_status(
            &mut evaluation_service,
            &wrapped_status,
            actual_status,
        ).await;
    }
}

impl WorkerService {
    async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let evaluation_service = EvaluationClient::connect("http://[::1]:50051").await?;

        Ok(WorkerService {
            status: Arc::new(Mutex::new(FileStatus::new())),
            evaluation_service,
        })
    }
}

fn run_sandbox(config: SandboxConfiguration) -> Result<SandboxExecutionResult, Error> {
    let sandbox = SandboxImplementation::run(config)
        .map_err(|e| format_err!("Failed to create sandbox: {:?}", e))?;
    let res = sandbox
        .wait()
        .map_err(|e| format_err!("Failed to wait sandbox: {:?}", e))?;
    Ok(res)
}

fn map_used_resources(resource_used: ResourceUsage) -> Resources {
    let secs = resource_used.user_cpu_time as u64;
    let nanos = (resource_used.user_cpu_time.fract() * 1_000_000_000f64) as u32;
    Resources {
        time: Duration { secs, nanos },
        memory_bytes: resource_used.memory_usage,
    }
}

fn read_score() -> Result<Score, Error> {
    let score = std::fs::read_to_string(PathBuf::from("/tmp/tabox/checker/checker-stdout.txt"))?
        .parse::<f64>()?;
    Ok(Score { score })
}

#[tonic::async_trait]
impl Worker for WorkerService {
    // before returning, clean entirely the directory tmp/tabox/ but not the checkers executables
    async fn evaluate_submission(
        &self,
        request: Request<EvaluateSubmissionRequest>,
    ) -> Result<Response<EvaluateSubmissionResponse>, Status> {
        let request_inner = request.into_inner();

        if let ProgrammingLanguage::None = request_inner.source.lang() {
            return Err(Status::invalid_argument(
                "The source code has None programming language",
            ));
        }

        let get_problem_request = GetProblemRequest {
            problem_id: request_inner.problem_id,
        };

        let mut evaluation_service = MockEvaluation::default();
        init_evaluation_service(&mut evaluation_service, request_inner.problem_id);

        let problem_metadata = evaluation_service
            .get_problem(Request::new(get_problem_request))
            .await?
            .into_inner()
            .info;

        let compilation_config =
            get_compilation_config(problem_metadata.clone(), request_inner.source)
                .map_err(|e| Status::aborted(e.to_string()))?;
        println!("Compilation config: {:?}", compilation_config);

        let compilation_res = run_sandbox(compilation_config.clone())
            .map_err(|e| Status::aborted(e.to_string()))?; // problems with sandbox
        
        println!(
            "Compilation successfull? {}, exit status {:?}",
            compilation_res.status.success(),
            compilation_res.status
        );

        println!(
            "{}",
            std::fs::read_to_string(PathBuf::from("/tmp/tabox/compilation/stderr.txt"))
                .expect("cannot read")
        );

        if !compilation_res.status.success() {
            // unsuccessfull compilation
            let is_mle = compilation_config.memory_limit.map_or(false, |memory_limit| {
                memory_limit < compilation_res.resource_usage.memory_usage
            });
            let is_tle = compilation_config.time_limit.map_or(false, |time_limit| {
                time_limit
                    < ((compilation_res.resource_usage.user_cpu_time * 1_000_000_000f64)
                        as u64)
            });
            return Ok(Response::new(EvaluateSubmissionResponse {
                compilation_result: CompilationResult {
                    outcome: if is_tle {
                        compilation_result::Outcome::Tle as i32
                    } else if is_mle {
                        compilation_result::Outcome::Mle as i32
                    } else {
                        compilation_result::Outcome::Rte as i32
                    },
                    used_resources: map_used_resources(compilation_res.resource_usage),
                },
                testcase_results: vec![],
            }));
        }
        // successfull compilation
        
        let mut evaluation_response = EvaluateSubmissionResponse {
            compilation_result: CompilationResult {
                outcome: compilation_result::Outcome::Success as i32,
                used_resources: map_used_resources(compilation_res.resource_usage),
            },
            testcase_results: vec![], // yet to be evaluated
        };

        println!(
            "Executable exists? {}",
            PathBuf::from(join_path_str(
                PathBuf::from("/tmp/tabox/compilation"),
                EXECUTABLE_NAME.to_string(),
            ))
            .exists()
        );

        println!(
            "Compilation directory exists? {}",
            PathBuf::from("/tmp/tabox/compilation").exists()
        );

        let input_file_path = PathBuf::from("/tmp/tabox/execution/stdin.txt");
        let output_file_path = PathBuf::from("/tmp/tabox/execution/stdout.txt");

        let exec_config =
            get_execution_config(problem_metadata.clone(), input_file_path.clone());

        let outer_problem_id = request_inner.problem_id;
        let wrapped_status_copy = &Arc::clone(&self.status);

        evaluation_response.testcase_results = wrapped_status_copy
            .lock()
            .await
            .testcases
            .keys()
            .filter(|(problem_id, _)| *problem_id == outer_problem_id)
            .map(|(problem_id, testcase_id)| {

                // save the testcase input in the file
                let testcase_dir = get_testcase_dir_path(*problem_id, *testcase_id);
                std::fs::copy(testcase_dir.join("input.txt"), input_file_path.clone())
                    .expect("Failed to copy the testcase input into the working directory.");
                std::fs::copy(testcase_dir.join("output.txt"), output_file_path.clone())
                    .expect("Failed to copy the testcase input into the working directory.");

                println!("Running the execution in the sandbox");

                let execution_res = run_sandbox(exec_config.clone())
                    .map_err(|e| {
                        panic!("{:?}", Status::aborted(e.to_string()));
                    })
                    .unwrap();

                println!("Execution in the sandbox terminated");

                println!(
                    "Execution directory exists? {}",
                    PathBuf::from("/tmp/tabox/execution").exists()
                );
                println!(
                    "Execution successfull? {}, exit status {:?} {:?}",
                    execution_res.status.success(),
                    execution_res.status,
                    execution_res.status.signal_name()
                );
                println!(
                    "content of stdin.txt: \"{}\"",
                    std::fs::read_to_string(input_file_path.clone()).expect("cannot read")
                );
                println!(
                    "content of stdout.txt: \"{}\"",
                    std::fs::read_to_string(output_file_path.clone()).expect("cannot read")
                );

                if !execution_res.status.success() {
                    let is_mle = exec_config.memory_limit.map_or(false, |memory_limit| {
                        memory_limit < execution_res.resource_usage.memory_usage
                    });
                    let is_tle = exec_config.time_limit.map_or(false, |time_limit| {
                        time_limit
                            < ((execution_res.resource_usage.user_cpu_time * 1_000_000_000f64)
                                as u64)
                    });
                    return TestcaseResult {
                        outcome: {
                            if is_tle {
                                Outcome::Tle as i32
                            } else if is_mle {
                                Outcome::Mle as i32
                            } else {
                                Outcome::Rte as i32
                            }
                        },
                        score: Score { score: 0f64 },
                        used_resources: map_used_resources(execution_res.resource_usage),
                        id: *testcase_id,
                    };
                }

                println!("Successfull!");

                let checker_exec_config = 
                    get_checker_execution_config(problem_metadata.clone(), testcase_dir.join("output.txt"))
                        .map_err(|e| {
                            panic!("{:?}", Status::aborted(e.to_string()));
                        })
                        .unwrap();

                // run sandbox with checker to check if the result is correct
                let checker_res = run_sandbox(checker_exec_config)
                    .map_err(|e| {
                        panic!("{:?}", Status::aborted(e.to_string()));
                    })
                    .unwrap();

                if checker_res.status.success() {
                    // Pre: checker-output.txt contains just an f64 number
                    let score = read_score();
                    TestcaseResult {
                        outcome: match score {
                            Ok(_) => Outcome::Ok as i32,
                            Err(_) => Outcome::CheckerError as i32
                        },
                        score: score.unwrap_or(Score { score: 0f64 }), 
                        used_resources: map_used_resources(execution_res.resource_usage),
                        id: *testcase_id,
                    }
                } else {
                    // code returned by the checker execution is not zero
                    TestcaseResult {
                        outcome: Outcome::CheckerError as i32,
                        score: Score { score: 0f64 },
                        used_resources: map_used_resources(execution_res.resource_usage),
                        id: *testcase_id,
                    }
                }
            })
            .collect::<Vec<TestcaseResult>>();
        Ok(Response::new(evaluation_response))
    }

    async fn update_testcase(
        &self,
        _request: Request<UpdateTestcaseRequest>,
    ) -> Result<Response<UpdateTestcaseResponse>, Status> {
        todo!()
    }

    async fn update_source(
        &self,
        _request: Request<UpdateSourceRequest>,
    ) -> Result<Response<UpdateSourceResponse>, Status> {
        // compile the source code of the checker inside a sandbox and
        // save the result into the directory tmp/tabox/checkers
        todo!()
    }
}

fn init_evaluation_service(evaluation_service: &mut MockEvaluation, problem_id: u64) {
    evaluation_service.get_problem_set(GetProblemResponse {
        info: Problem {
            id: problem_id,
            scoring: scoring::Problem {
                method: scoring::problem::Method::MaxSum as i32,
            },
            r#type: problem::Type::Other as i32,
            execution_limits: Resources {
                time: Duration {
                    secs: 2u64,
                    nanos: 0u32,
                },
                memory_bytes: 256u64 * 1024u64 * 1024u64,
            },
            compilation_limits: Resources {
                time: Duration {
                    secs: 2u64,
                    nanos: 0u32,
                },
                memory_bytes: 256u64 * 1024u64 * 1024u64,
            },
            subtasks: vec![],
        },
    });

    evaluation_service.get_problem_testcases_set(GetProblemTestcasesResponse {
        testcases: vec![Testcase {
            id: 1,
            input: Some("1".as_bytes().to_vec()),
            output: None,
        }],
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print stacktraces useful to debug sandbox failures.
    std::env::set_var("RUST_BACKTRACE", "1");

    let addr: _ = "127.0.0.1:50051".parse()?;
    let worker_service = WorkerService::new().await?;
    let status_copy = Arc::clone(&worker_service.status);
    let evaluation_service_copy = worker_service.evaluation_service.clone();

    let _pull_thread_handler = spawn(move || 
        pull_join_handler_action(
            evaluation_service_copy, 
            status_copy
        ));

    let request = EvaluateSubmissionRequest {
        problem_id: 1,
        source: Source {
            lang: ProgrammingLanguage::Cpp as i32,
            code: "#include<iostream>\nint main() { int x; std::cin >> x; std::cout << \"Hey there! \" << x << \" is a great number\"; return 0; }".as_bytes().to_vec()
            // code: "#include<iostream>\nint main() { std::cout << \"Hey there!\"; return 0; }".as_bytes().to_vec()
            // lang: ProgrammingLanguage::Rust as i32,
            // code: "fn main() { println!(\"Hey there!\"); }".as_bytes().to_vec()
        },
    };
    println!("Request is:\n\n{:?}\n\n", request.clone());
    println!(
        "{:?}",
        worker_service
            .evaluate_submission(Request::new(request))
            .await?
            .into_inner()
    );
    println!(
        "Compilation directory exists? {}",
        PathBuf::from("/tmp/tabox/compilation").exists()
    );
    println!(
        "Execution directory exists? {}",
        PathBuf::from("/tmp/tabox/execution").exists()
    );
    println!(
        "Checker directory exists? {}",
        PathBuf::from("/tmp/tabox/checker").exists()
    );
    println!("Finished hardcoded example");

    println!("Starting a worker server");
    Server::builder()
        .add_service(WorkerServer::new(worker_service))
        .serve(addr)
        .await?;

    Ok(())
}
