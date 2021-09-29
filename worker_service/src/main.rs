use failure::{format_err, Error};
use protos::{
    common::{self, Duration, ProgrammingLanguage, Resources, Source},
    evaluation::{compilation_result, CompilationResult, TestcaseResult},
    scoring,
    service::{
        evaluation::{
            evaluation_server::Evaluation, problem, GetProblemEvaluationFileRequest,
            GetProblemRequest, GetProblemResponse, GetProblemTestcasesRequest, MockEvaluation,
            Problem, Testcase, GetProblemTestcasesResponse
        },
        worker::{
            worker_server::{Worker, WorkerServer},
            EvaluateSubmissionRequest, EvaluateSubmissionResponse, UpdateSourceRequest,
            UpdateSourceResponse, UpdateTestcaseRequest, UpdateTestcaseResponse,
        },
    },
    utils::{get_local_address, Service},
    worker,
};
use std::path::{Path, PathBuf};
use tabox::{
    configuration::SandboxConfiguration,
    result::SandboxExecutionResult,
    syscall_filter::{SyscallFilter, SyscallFilterAction},
    SandboxImplementation,
};
use tabox::{result::ResourceUsage, Sandbox};
use tonic::{transport::Server, Code, Request, Response, Status};
use which::which;

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
    // update-alternatives stuff, sometimes the executables are symlinked here
    "/etc/alternatives/",
    "/var/lib/dpkg/alternatives/",
    // required by texlive on Ubuntu
    "/var/lib/texmf/",
];

pub struct WorkerService {}

impl WorkerService {
    pub fn new() -> Self {
        WorkerService {}
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

fn save_source_code(source: Source, path: PathBuf) -> Result<(), Error> {
    // Save source.code to path.
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|io_error| {
        format_err!(
            "While creating parent dir for sandbox-accessible source code file: {}",
            io_error.to_string()
        )
    })?;
    std::fs::write(path, source.code).map_err(|io_error| {
        format_err!(
            "While creating sandbox-accessible source code file: {}",
            io_error.to_string()
        )
    })
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
        .wall_time_limit(30 * problem_metadata.compilation_limits.time.secs)
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
        .syscall_filter(SyscallFilter {
            default_action: SyscallFilterAction::Allow,
            // Overwrites the default behaviour for the specified rules.
            rules: vec![],
        })
        .uid(1000) // Configured in the Dockerfile.
        .gid(1000);

    for dir in READABLE_DIRS {
        if Path::new(dir).is_dir() {
            compilation_config.mount(dir, dir, false);
        }
    }

    save_source_code(
        source,
        PathBuf::from(join_path_str(compilation_dir, source_code_file)),
    )?;

    Ok(compilation_config.build())
}

fn get_execution_config(
    problem_metadata: Problem,
    input_file_path: PathBuf
) -> Result<SandboxConfiguration, Error> {
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
        /*.stdout(PathBuf::from(join_path_str(
            execution_dir.clone(),
            String::from("stdout.txt"),
        ))) */
        .syscall_filter(SyscallFilter {
            default_action: SyscallFilterAction::Kill,
            // Overwrites the default behaviour for the specified rules.
            rules: vec![],
        })
        .uid(1000) // Configured in the Dockerfile.
        .gid(1000);

    for dir in READABLE_DIRS {
        if Path::new(dir).is_dir() {
            execution_config.mount(dir, dir, false);
        }
    }

    Ok(execution_config.build())
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
            return Err(Status::new(
                Code::InvalidArgument,
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

        // TODO NEED TO SET UP THE DIRECTORIES MANUALLY, check this file entirely:
        // https://github.com/edomora97/task-maker-rust/blob/master/task-maker-exec/src/sandbox.rs

        let compilation_config = get_compilation_config(problem_metadata.clone(), request_inner.source)
            .map_err(|e| Status::new(Code::Aborted, e.to_string()))?;
        println!("Compilation config: {:?}", compilation_config);
        // println!("Run results: {:?}", run_sandbox(compilation_config));
        // println!("{0} is an existing directory? {1}", "/tmp/tabox/compilation", PathBuf::from("/tmp/tabox/compilation").exists());
        // return Err(Status::new(Code::Unimplemented, "Stopped here"));

        let mut evaluation_response: EvaluateSubmissionResponse;
        match run_sandbox(compilation_config) {
            Ok(res) => {
                println!("Compilation successfull? {}, exit status {:?}", res.status.success(), res.status);
                evaluation_response = EvaluateSubmissionResponse {
                    compilation_result: CompilationResult {
                        outcome: compilation_result::Outcome::Success as i32,
                        used_resources: map_used_resources(res.resource_usage),
                    },
                    testcase_results: vec![], // yet to be evaluated
                }
            }
            Err(e) => {
                return Ok(Response::new(EvaluateSubmissionResponse {
                    compilation_result: CompilationResult {
                        outcome: todo!(),        // deduce from e
                        used_resources: todo!(), // not applicable
                    },
                    testcase_results: vec![],
                }));
            }
        }

        println!("Executable exists? {}", PathBuf::from(join_path_str(
            PathBuf::from("/tmp/tabox/compilation"),
            EXECUTABLE_NAME.to_string(),
        )).exists());
        println!("Compilation directory exists? {}", PathBuf::from("/tmp/tabox/compilation").exists());

        evaluation_service.get_problem_testcases_set(GetProblemTestcasesResponse {
            testcases: vec![Testcase {id: 1, input: Some(vec![]), output: None}]
        });
        let testcases = evaluation_service
            .get_problem_testcases(Request::new(GetProblemTestcasesRequest {
                problem_id: request_inner.problem_id,
            }))
            .await?
            .into_inner()
            .testcases;

        evaluation_response.testcase_results = testcases
            .iter()
            .map(|testcase: &Testcase| {
                let input_file_path = PathBuf::from("/tmp/tabox/execution/stdin.txt");
                save_file(testcase.input.clone().unwrap(), input_file_path.clone()).unwrap();
                // TODO create execution_config once and always use the same
                let execution_config = 
                    get_execution_config(problem_metadata.clone(), input_file_path.clone())
                    .map_err(|e| {
                        panic!("{:?}", Status::new(Code::Aborted, e.to_string()));
                    })
                    .unwrap();
                
                let execution_res = run_sandbox(execution_config)
                    .map_err(|e| {
                        panic!("{:?}", Status::new(Code::Aborted, e.to_string()));
                    })
                    .unwrap();

                println!("{}", std::fs::read_to_string(input_file_path).expect("cannot read"));
                
                todo!()
                // write testcase to file (we must allow the sandbox to access that file)
                // run sandbox to execute this testcase
                // if reached a result in time, run sandbox to run the checker on the result
            })
            .collect::<Vec<TestcaseResult>>();
        Ok(Response::new(evaluation_response))
        // fetch problem data:
        //     - invoke get_testcases from Evaluation Service to get all the testcases
        //     - invoke get_problem from Evaluation Service to get problem metadata
        //       (in particular we need the compilation/execution limits)
        //     - do we need the evaluation file?
        // populate configuration based on data  ^
        // compilation
        // run the execution inside the sandbox
        // get the sandbox results and build the worker response
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
            lang: ProgrammingLanguage::Rust as i32,
            code: "fn main() { println!(\"Hey there!\"); }".as_bytes().to_vec()
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
    println!("Finished hardcoded example");

    println!("Starting a worker server");
    Server::builder()
        .add_service(WorkerServer::new(worker_service))
        .serve(addr)
        .await?;

    Ok(())
}
