use failure::{format_err, Error};
use protos::{
    common::{ProgrammingLanguage, Source},
    service::evaluation::Problem
};
use std::path::{Path, PathBuf};
use tabox::{configuration::SandboxConfiguration, syscall_filter::SyscallFilter};

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

pub const SOURCE_CODE_NAME: &str = "main";
pub const EXECUTABLE_NAME: &str = "executable";

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

pub fn get_checker_execution_config(
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
pub fn join_path_str(path1: PathBuf, path2: String) -> String {
    path1.join(path2).into_os_string().into_string().unwrap()
}

pub fn get_extension(lang: ProgrammingLanguage) -> String {
    match lang {
        ProgrammingLanguage::None => panic!(),
        ProgrammingLanguage::Rust => String::from(".rs"),
        ProgrammingLanguage::Cpp => String::from(".cpp"),
    }
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

pub fn get_compiler(lang: ProgrammingLanguage) -> PathBuf {
    match lang {
        ProgrammingLanguage::None => panic!(),
        ProgrammingLanguage::Rust => PathBuf::from("/usr/local/cargo/bin/rustc"),
        ProgrammingLanguage::Cpp => PathBuf::from("/usr/bin/g++"),
    }
}
