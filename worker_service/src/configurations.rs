use failure::{format_err, Error};
use protos::{
    common::{ProgrammingLanguage, Source},
    service::evaluation::{evaluation_file, Problem},
};
use std::path::{Path, PathBuf};
use tabox::{configuration::SandboxConfiguration, syscall_filter::SyscallFilter};

use crate::{ProblemId, TestcaseId};

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

pub const SOURCE_CODE_NAME: &str = "main";
pub const EXECUTABLE_NAME: &str = "executable";
pub const CHECKER_EXECUTABLE_NAME: &str = "checker-executable";

pub fn get_extension(lang: ProgrammingLanguage) -> String {
    match lang {
        ProgrammingLanguage::None => panic!(),
        ProgrammingLanguage::Rust => String::from(".rs"),
        ProgrammingLanguage::Cpp => String::from(".cpp"),
    }
}

pub fn get_compiler(lang: ProgrammingLanguage) -> PathBuf {
    match lang {
        ProgrammingLanguage::None => panic!(),
        ProgrammingLanguage::Rust => PathBuf::from("/usr/local/cargo/bin/rustc"),
        ProgrammingLanguage::Cpp => PathBuf::from("/usr/bin/g++"),
    }
}

pub fn get_testcase_dir_path(problem_id: ProblemId, testcase_id: TestcaseId) -> PathBuf {
    get_problem_dir_path(problem_id).join(format!("testcase{}", testcase_id))
}

pub fn get_problem_dir_path(problem_id: ProblemId) -> PathBuf {
    PathBuf::from(format!("/tmp/tabox-utils/problem{}", problem_id))
}

pub fn get_checker_executable_name(checker_type: evaluation_file::Type) -> String {
    format!("{}-{}", CHECKER_EXECUTABLE_NAME, checker_type.to_string()).to_string()
}

pub fn get_checker_source_name(
    checker_type: evaluation_file::Type,
    lang: ProgrammingLanguage,
) -> String {
    format!(
        "checker-{}{}",
        checker_type.to_string(),
        get_extension(lang)
    )
}

pub fn join_path_str(path1: PathBuf, path2: String) -> String {
    path1.join(path2).into_os_string().into_string().unwrap()
}

pub fn save_file(content: Vec<u8>, path: PathBuf) -> Result<(), Error> {
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

pub fn get_compilation_config(
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

pub fn get_execution_config(
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

pub fn get_checker_compilation_config(
    problem_id: ProblemId,
    checker_type: evaluation_file::Type,
    source: Source,
) -> Result<SandboxConfiguration, Error> {
    let mut compilation_config = SandboxConfiguration::default();

    let problem_dir = get_problem_dir_path(problem_id);
    let tmp_compilation_dir = problem_dir.join("checker-compilation");
    let checker_source_name = get_checker_source_name(checker_type, source.lang());

    compilation_config
        .mount(problem_dir.clone(), problem_dir.clone(), true) // where to save executable
        .mount(
            tmp_compilation_dir.clone(),
            tmp_compilation_dir.clone(),
            true,
        )
        .working_directory(tmp_compilation_dir.clone())
        .wall_time_limit(10)
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .executable(get_compiler(source.lang()))
        .arg("-o")
        .arg(join_path_str(
            problem_dir,
            get_checker_executable_name(checker_type),
        ))
        .arg(join_path_str(
            tmp_compilation_dir.clone(),
            checker_source_name.clone(),
        ))
        .stderr(PathBuf::from(join_path_str(
            tmp_compilation_dir.clone(),
            String::from("stderr.txt"),
        )))
        .stdout(PathBuf::from(join_path_str(
            tmp_compilation_dir.clone(),
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
        PathBuf::from(join_path_str(tmp_compilation_dir, checker_source_name)),
    )?;

    Ok(compilation_config.build())
}

pub fn get_checker_execution_config(
    problem_metadata: Problem,
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
        .executable(
            join_path_str(
                get_problem_dir_path(problem_metadata.id),
                get_checker_executable_name(evaluation_file::Type::Checker),
            ), // TODO generify this
        )
        .arg(format!(
            "{}",
            correct_output_file_path
                .into_os_string()
                .into_string()
                .unwrap()
        )) // pass the path to the correct output file as command-line argument
        .stdin(PathBuf::from(join_path_str(
            execution_dir.clone(),
            String::from("stdout.txt"),
        ))) // the output of the participant's solution
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
