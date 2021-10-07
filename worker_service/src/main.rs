use failure::{format_err, Error};
use protos::{
    common::{self, Duration, ProgrammingLanguage, Resources, Score, Source, Timestamp},
    evaluation::{compilation_result, testcase_result::Outcome, CompilationResult, TestcaseResult},
    scoring,
    service::{
        evaluation::{
            evaluation_server::Evaluation, problem, GetProblemEvaluationFileRequest,
            GetProblemRequest, GetProblemResponse, GetProblemTestcasesRequest,
            GetProblemTestcasesResponse, MockEvaluation, Problem, Testcase,
        },
        worker::{
            worker_server::{Worker, WorkerServer},
            EvaluateSubmissionRequest, EvaluateSubmissionResponse, UpdateSourceRequest,
            UpdateSourceResponse, UpdateTestcaseRequest, UpdateTestcaseResponse,
        },
    },
    utils::{get_local_address, Service},
    worker::{self, Testcase},
};
use std::{
    collections::HashMap,
    iter::Map,
    path::{Path, PathBuf},
    process::ExitStatus,
    thread::{sleep, spawn, JoinHandle},
};
use tabox::{
    configuration::SandboxConfiguration,
    result::SandboxExecutionResult,
    syscall_filter::{SyscallFilter, SyscallFilterAction},
    SandboxImplementation,
};
use tabox::{result::ResourceUsage, Sandbox};
use tonic::{transport::Server, Code, Request, Response, Status};

mod languages_info;

const SOURCE_CODE_NAME: &str = "main";
const EXECUTABLE_NAME: &str = "executable";

// The list of all the system-wide readable directories inside the sandbox.
// TODO: probably not all of these are needed, remove the unneeded.
const READABLE_DIRS: &[&str] = &[
    "/lib",
    "/lib64",
    "/usr",
    "/bin",
    "/opt",
    // "/proc",
    // update-alternatives stuff, sometimes the executables are symlinked here
    "/etc/alternatives/",
    "/var/lib/dpkg/alternatives/",
    // required by texlive on Ubuntu
    "/var/lib/texmf/",
];

struct FileStatus {
    testcases: HashMap<u64, Timestamp>,
    checkers: HashMap<u64, Timestamp>,
}

struct EvaluationFileStatus {
    // vectors of id and correspondent timestamp
    testcases: Vec<(u64, Timestamp)>,
    checkers: Vec<(u64, Timestamp)>,
}

pub struct WorkerService {
    status: FileStatus,
    pull_join_handler: JoinHandle<()>,
}

fn update_testcase_helper(
    worker: &WorkerService,
    testcase_id: u64,
) -> Result<Response<UpdateTestcaseResponse>, Status> {
    worker.update_testcase(Request::new(UpdateTestcaseRequest {
        tc: worker::Testcase {
            problem_id: todo!(), // TODO change this RPC message to PULL mode!!!!!
            testcase_id,
            testcase: None, // TODO remove
        },
    }))
}

fn update_checker_helper(
    worker: &WorkerService,
    problem_id: u64,
) -> Result<Response<UpdateSourceResponse>, Status> {
    worker.update_source(Request::new(UpdateSourceRequest {
        file: worker::SourceFile {
            problem_id,
            r#type: todo!(), // do I really need this? In any case the compilation is the same...
            source: todo!(), // TODO remove
        },
    }))
}

fn diff_and_update_status(worker: &WorkerService, actual_status: EvaluationFileStatus) {
    actual_status
        .testcases
        .iter()
        .for_each(|(testcase_id, actual_timestamp)| {
            // insert the new one and get the older value associated to the key
            let old_timestamp = worker
                .status
                .testcases
                .insert(testcase_id, actual_timestamp);

            if (old_timestamp.is_none() || old_timestamp.unwrap() < actual_timestamp) {
                // try pulling until it succeeds
                loop {
                    if let Ok(_) = update_testcase_helper(worker, testcase_id) {
                        break;
                    }
                }
            }
        });
    
    actual_status
        .checkers
        .iter()
        .for_each(|(problem_id, actual_timestamp)| {
            // insert the new one and get the older value associated to the key
            let old_timestamp = worker
                .status
                .checkers
                .insert(problem_id, actual_timestamp);

            if (old_timestamp.is_none() || old_timestamp.unwrap() < actual_timestamp) {
                // pull until it succeeds
                loop {
                    if let Ok(_) = update_checker_helper(worker, problem_id) {
                        break;
                    }
                }
            }
        });
}

impl WorkerService {
    pub fn new() -> Self {
        let pull_join_handler: JoinHandle<()>;
        let worker = WorkerService {
            status: FileStatus {
                testcases: HashMap::new(),
                checkers: HashMap::new(),
            },
            pull_join_handler,
        };

        // kill this thread when the worker goes down using the pull_join_handler
        pull_join_handler = spawn(|| {
            loop {
                // sleep 30 secs
                sleep(std::time::Duration::new(30, 0));

                // pull status. TODO actually do the pull
                let actual_status: EvaluationFileStatus;

                // diff and update
                diff_and_update_status(&worker, actual_status);
            }
        });

        // if this fails, substitute pull_join_handler with worker.pull_join_handler
        // on the spawn line
        assert!(pull_join_handler == worker.pull_join_handler);

        worker
    }
}

fn join_path_str(path1: PathBuf, path2: String) -> String {
    path1.join(path2).into_os_string().into_string().unwrap()
}

fn get_extension(lang: ProgrammingLanguage) -> String {
    match lang {
        ProgrammingLanguage::None => panic!(),
        ProgrammingLanguage::Rust => String::from(".rs"),
        ProgrammingLanguage::Cpp => String::from(".cpp"),
    }
}

fn save_file(content: Vec<u8>, path: PathBuf) -> Result<(), Error> {
    // Save content to path.
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|io_error| {
        format_err!(
            "While creating parent dir for sandbox-accessible file: {}",
            io_error.to_string()
        )
    })?;
    std::fs::write(path, content).map_err(|io_error| {
        format_err!(
            "While creating sandbox-accessible file: {}",
            io_error.to_string()
        )
    })
}

fn get_compiler(lang: ProgrammingLanguage) -> PathBuf {
    match lang {
        ProgrammingLanguage::None => panic!(),
        ProgrammingLanguage::Rust => PathBuf::from("/usr/local/cargo/bin/rustc"),
        ProgrammingLanguage::Cpp => PathBuf::from("/usr/bin/g++"),
    }
}

fn get_compilation_config(
    problem_metadata: Problem,
    source: Source,
) -> Result<SandboxConfiguration, Error> {
    let mut compilation_config = SandboxConfiguration::default();

    let compilation_dir = PathBuf::from("/tmp/tabox/compilation");
    let source_code_file = format!("{}{}", SOURCE_CODE_NAME, get_extension(source.lang()));

    compilation_config
        .mount(compilation_dir.clone(), compilation_dir.clone(), true)
        .working_directory(compilation_dir.clone())
        .memory_limit(problem_metadata.compilation_limits.memory_bytes)
        .time_limit(problem_metadata.compilation_limits.time.secs)
        .wall_time_limit(5 * problem_metadata.compilation_limits.time.secs)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .executable(get_compiler(source.lang()))
        .arg("-o")
        .arg(EXECUTABLE_NAME)
        .arg(join_path_str(
            compilation_dir.clone(),
            source_code_file.clone(),
        ))
        .stderr(PathBuf::from(join_path_str(
            compilation_dir.clone(),
            String::from("stderr.txt"),
        )))
        .stdout(PathBuf::from(join_path_str(
            compilation_dir.clone(),
            String::from("stdout.txt"),
        )))
        .uid(1000) // Configured in the Dockerfile.
        .gid(1000);

    for dir in READABLE_DIRS {
        if Path::new(dir).is_dir() {
            compilation_config.mount(dir, dir, false);
        }
    }

    // save source code into a sandbox-accessible file
    save_file(
        source.code,
        PathBuf::from(join_path_str(compilation_dir, source_code_file)),
    )?;

    Ok(compilation_config.build())
}

fn get_execution_config(
    problem_metadata: Problem,
    input_file_path: PathBuf,
) -> SandboxConfiguration {
    let mut execution_config = SandboxConfiguration::default();

    let compilation_dir = PathBuf::from("/tmp/tabox/compilation");
    let execution_dir = PathBuf::from("/tmp/tabox/execution");

    execution_config
        .mount(execution_dir.clone(), execution_dir.clone(), true)
        .mount(compilation_dir.clone(), compilation_dir.clone(), false) // to read the executable
        .working_directory(execution_dir.clone())
        .memory_limit(problem_metadata.execution_limits.memory_bytes)
        .time_limit(problem_metadata.execution_limits.time.secs)
        .wall_time_limit(5 * problem_metadata.execution_limits.time.secs)
        .executable(PathBuf::from(join_path_str(
            compilation_dir.clone(),
            EXECUTABLE_NAME.to_string(),
        )))
        .stdin(input_file_path)
        .stdout(PathBuf::from(join_path_str(
            execution_dir.clone(),
            String::from("stdout.txt"),
        )))
        .syscall_filter(SyscallFilter::build(false, false))
        .uid(1000) // Configured in the Dockerfile.
        .gid(1000);

    for dir in READABLE_DIRS {
        if Path::new(dir).is_dir() {
            execution_config.mount(dir, dir, false);
        }
    }

    execution_config.build()
}

fn get_checker_execution_config(
    problem_metadata: Problem,
    output_file_path: PathBuf,
    correct_output_file_path: PathBuf,
) -> Result<SandboxConfiguration, Error> {
    let mut checker_execution_config = SandboxConfiguration::default();

    let execution_dir = PathBuf::from("/tmp/tabox/execution");
    let checker_dir = PathBuf::from("/tmp/tabox/checker");

    checker_execution_config
        .mount(execution_dir.clone(), execution_dir.clone(), false) // to read the execution output file
        .mount(checker_dir.clone(), checker_dir.clone(), true)
        .working_directory(checker_dir.clone())
        .wall_time_limit(3 * problem_metadata.execution_limits.time.secs) // all the stuff that the checker reads must have also been written within the time limit
        // .executable(todo!("path to checker executable"))
        .arg(format!("{}", correct_output_file_path.display()))
        .stdin(output_file_path)
        // how to set another stdin ?? just give him access to the file and pass it as CLI argument (see 2 lines above)
        .stdout(PathBuf::from(join_path_str(
            checker_dir.clone(),
            String::from("checker-stdout.txt"),
        )))
        .syscall_filter(SyscallFilter::build(false, false))
        .uid(1000) // Configured in the Dockerfile.
        .gid(1000);

    // all necessary?
    for dir in READABLE_DIRS {
        if Path::new(dir).is_dir() {
            checker_execution_config.mount(dir, dir, false);
        }
    }

    Ok(checker_execution_config.build())
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
                .map_err(|e| Status::new(Code::Aborted, e.to_string()))?;
        println!("Compilation config: {:?}", compilation_config);

        let mut evaluation_response: EvaluateSubmissionResponse;

        match run_sandbox(compilation_config) {
            Ok(res) => {
                println!(
                    "Compilation successfull? {}, exit status {:?}",
                    res.status.success(),
                    res.status
                );

                println!(
                    "{}",
                    std::fs::read_to_string(PathBuf::from("/tmp/tabox/compilation/stderr.txt"))
                        .expect("cannot read")
                );

                evaluation_response = EvaluateSubmissionResponse {
                    compilation_result: CompilationResult {
                        outcome: compilation_result::Outcome::Success as i32, // Nope! Depends on res.status
                        used_resources: map_used_resources(res.resource_usage),
                    },
                    testcase_results: vec![], // yet to be evaluated
                }
            }
            Err(e) => {
                // problems with sandbox
                return Err(Status::aborted(e.to_string()));
            }
        }

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

        evaluation_service.get_problem_testcases_set(GetProblemTestcasesResponse {
            testcases: vec![Testcase {
                id: 1,
                input: Some("1".as_bytes().to_vec()),
                output: None,
            }],
        });
        let testcases = evaluation_service
            .get_problem_testcases(Request::new(GetProblemTestcasesRequest {
                problem_id: request_inner.problem_id,
            }))
            .await?
            .into_inner()
            .testcases;

        let input_file_path = PathBuf::from("/tmp/tabox/execution/stdin.txt");
        let output_file_path = PathBuf::from("/tmp/tabox/execution/stdout.txt");

        let execution_config =
            get_execution_config(problem_metadata.clone(), input_file_path.clone());

        evaluation_response.testcase_results = testcases
            .iter()
            .map(|testcase: &Testcase| {
                // save the testcase input in the file
                save_file(testcase.input.clone().unwrap(), input_file_path.clone()).unwrap();

                println!("Running the execution in the sandbox");

                let execution_res = run_sandbox(execution_config.clone())
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
                    let is_mle = execution_config.memory_limit.map_or(false, |memory_limit| {
                        memory_limit < execution_res.resource_usage.memory_usage
                    });
                    let is_tle = execution_config.time_limit.map_or(false, |time_limit| {
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
                        id: testcase.id,
                    };
                }

                println!("Successfull!");

                // run sandbox with checker to check if the result is correct
                todo!("Checker execution to be done");
                let checker_config = SandboxConfiguration::default();

                let checker_res = run_sandbox(checker_config.clone())
                    .map_err(|e| {
                        panic!("{:?}", Status::aborted(e.to_string()));
                    })
                    .unwrap();

                if checker_res.status.success() {
                    // Pre: checker-output.txt contains just an f64 number
                    TestcaseResult {
                        outcome: Outcome::Ok as i32,
                        score: todo!(), // read from checker output
                        used_resources: map_used_resources(execution_res.resource_usage),
                        id: testcase.id,
                    }
                } else {
                    // map sandbox result status to execution outcome
                    TestcaseResult {
                        outcome: {
                            // CHECKER ERROR directly ?
                            match checker_res.status {
                                tabox::result::ExitStatus::ExitCode(_) => todo!(),
                                tabox::result::ExitStatus::Signal(_) => todo!(),
                                tabox::result::ExitStatus::Killed => todo!(),
                            }
                        },
                        score: todo!(), // read from checker output
                        used_resources: map_used_resources(execution_res.resource_usage),
                        id: testcase.id,
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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Print stacktraces useful to debug sandbox failures.
    std::env::set_var("RUST_BACKTRACE", "1");

    let addr: _ = "127.0.0.1:50051".parse()?;
    let worker_service = WorkerService::new();

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
