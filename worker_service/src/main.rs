use failure::{format_err, Error};
use protos::{
    common::{self, Duration, ProgrammingLanguage, Resources, Source},
    evaluation::{compilation_result, CompilationResult, TestcaseResult},
    scoring,
    service::{
        evaluation::{
            evaluation_server::Evaluation, problem, GetProblemEvaluationFileRequest,
            GetProblemRequest, GetProblemResponse, GetProblemTestcasesRequest, MockEvaluation,
            Problem, Testcase,
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
use which::which;
use std::path::{Path, PathBuf};
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

pub struct WorkerService {}

impl WorkerService {
    pub fn new() -> Self {
        WorkerService {}
    }
}

fn get_extension(lang: ProgrammingLanguage) -> String {
    match lang {
        ProgrammingLanguage::None => panic!(),
        ProgrammingLanguage::Rust => String::from(".rs"),
        ProgrammingLanguage::Cpp => String::from(".cpp"),
    }
}

fn save_source_code(source: Source, mut path: PathBuf) -> Result<(), Error> {
    let filename = format!("{}{}", SOURCE_CODE_NAME, get_extension(source.lang()));
    path.push(PathBuf::from(filename));
    // Save source.code to path.
    std::fs::create_dir_all(path.parent().unwrap()).map_err(|io_error| format_err!("While creating parent dir for sandbox-accessibile source code file: {}", io_error.to_string()))?;
    std::fs::write(path, source.code).map_err(|io_error| format_err!("While creating sandbox-accessibile source code file: {}", io_error.to_string()))
}

fn get_compiler_executable(lang: ProgrammingLanguage) -> PathBuf {
    match lang {
        ProgrammingLanguage::None => panic!(),
        // TODO: should this not just be rustc?
        ProgrammingLanguage::Rust => PathBuf::from("/tmp/tabox/compilation/compiler/rustc"),
        ProgrammingLanguage::Cpp => PathBuf::from("g++"),
    }
}

fn get_compilation_config(
    problem_metadata: Problem,
    source: Source,
) -> Result<SandboxConfiguration, Error> {
    let mut compilation_config = SandboxConfiguration::default();

    // Not very helpful. TODO: remove.
    // std::env::set_var("RUST_BACKTRACE", "1");  // Print stacktrace if failure in sandbox.

    compilation_config
        .working_directory(PathBuf::from("/tmp/tabox/compilation"))
        .executable(get_compiler_executable(source.lang()))
        .time_limit(problem_metadata.compilation_limits.time.secs)
        .memory_limit(problem_metadata.compilation_limits.memory_bytes)
        .arg("-o")
        .arg("executable")
        .arg(format!(
            "{}{}",
            SOURCE_CODE_NAME,
            get_extension(source.lang())
        ))
        //.wall_time_limit(5 * problem_metadata.compilation_limits.time.secs)
        .stderr(PathBuf::from(
            "/tmp/tabox/compilation/compilation_output.txt",
        ))
        .syscall_filter(
            SyscallFilter {
            // default behaviour if a system call is invoked
            // TODO: allow everything for now, but this shoudl be reduced as much as possible.
            default_action: SyscallFilterAction::Allow,
            // overwrites the default behaviour for the specified rules
            rules: vec![],
        }) // no syscall
        .uid(1000) // Configured in the Dockerfile.
        .gid(1000)
        .build();
        //.mount(PathBuf::new(), PathBuf::from("/tmp/tabox/compilation/compiler/"), true);

    // This line fails, need to find the right way to write into a sandbox owned repo?
    save_source_code(source, compilation_config.working_directory.clone())?;

    Ok(compilation_config)
}

// TODO: remove.
fn test_easy_config() -> Result<SandboxConfiguration, Error> {
    let mut compilation_config = SandboxConfiguration::default();
    std::fs::create_dir_all(PathBuf::from("/tmp/tabox/other")).unwrap();
    compilation_config
        .working_directory(PathBuf::from("/tmp/tabox/other"))
        .executable(which("echo").unwrap())
        .arg("\"test-echo-inside-sandbox\"")
        .arg(">> /tmp/tabox/other/out.txt")
        .syscall_filter(SyscallFilter {
            // default behaviour if a system call is invoked
            default_action: SyscallFilterAction::Kill,
            // overwrites the default behaviour for the specified rules
            // Example: allows echo
            rules: vec![("echo".to_string(), SyscallFilterAction::Allow)],
        })
        .mount(which("echo").unwrap(), which("echo").unwrap(), false)
        .mount(PathBuf::from("/tmp/tabox/other"), PathBuf::from("/tmp/tabox/other"), true)
        //.stdout(PathBuf::from("/tmp/tabox/other/out.txt"))
        .uid(1000) // see https://github.com/edomora97/task-maker-rust/blob/master/task-maker-exec/src/sandbox.rs#L367
        .gid(1000);

    Ok(compilation_config)
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

        let compilation_config = get_compilation_config(problem_metadata, request_inner.source)
            .map_err(|e| Status::new(Code::Aborted, e.to_string()))?;
        println!("Compilation config: {:?}", compilation_config);
        println!("Run results: {:?}", run_sandbox(compilation_config));
        return Err(Status::new(Code::Unimplemented, "Stopped here"));

        let mut evaluation_response: EvaluateSubmissionResponse;
        match run_sandbox(compilation_config) {
            Ok(res) => {
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

        let testcases = evaluation_service
            .get_problem_testcases(Request::new(GetProblemTestcasesRequest {
                problem_id: request_inner.problem_id,
            }))
            .await?
            .into_inner()
            .testcases;

        evaluation_response.testcase_results = testcases
            .iter()
            .map(|testcase| {
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
    // let addr: _ = "127.0.0.1:50051".parse()?;
    // let worker_service = WorkerService::new();
    
    // // std::fs::write(PathBuf::from("/tmp/pippo-puzza/test.txt"), "ciao".as_bytes())?;
    // println!("Starting a worker server");
    // println!(
    //     "{:?}",
    //     worker_service
    //         .evaluate_submission(Request::new(EvaluateSubmissionRequest {
    //             problem_id: 1,
    //             source: Source {
    //                 lang: ProgrammingLanguage::Rust as i32,
    //                 code: "fn main() {}".as_bytes().to_vec()
    //             },
    //         }))
    //         .await?
    //         .into_inner()
    // );
    // Server::builder()
    //     .add_service(WorkerServer::new(worker_service))
    //     .serve(addr)
    //     .await?;

    // Start child in an unshared environment
    let child_pid = unsafe {
        libc::syscall(
            libc::SYS_clone,
            // TODO: Setting any of the CLONE_* files make it fail.
            // We need to find a (Dockerfile) setup that allows them all.
            // libc::CLONE_NEWIPC,
            // libc::CLONE_NEWNET,
            // libc::CLONE_NEWNS,
            // libc::CLONE_NEWPID,
            // libc::CLONE_NEWUSER,
            // libc::CLONE_NEWUTS,
            libc::SIGCHLD,
            std::ptr::null::<libc::c_void>(),
        )
    } as libc::pid_t;

    if child_pid < 0 {
        println!("***************** ERROR ******************\nclone() error: errno is {}", nix::errno::Errno::last() as i32);
    }

    Ok(())
}
