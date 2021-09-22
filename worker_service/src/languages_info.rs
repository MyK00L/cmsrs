use std::path::PathBuf;

use protos::common::ProgrammingLanguage;

pub struct ProgrammingLanguageCompilationInfo {
    // compiler: PathBuf,
// compiler_args: Vec<String>,
// source_code_extension: String,
}

pub struct ProgrammingLanguageInfo {}

pub enum ProgrammingLanguageWithInfo {
    None,
    Rust(ProgrammingLanguageInfo),
    Cpp(ProgrammingLanguageInfo),
}

// NB: if the developer only updates the other enum, he will get a
//     compilation error because this match expression will miss an arm
impl From<ProgrammingLanguage> for ProgrammingLanguageWithInfo {
    fn from(lang: ProgrammingLanguage) -> Self {
        match lang {
            // use macros here instead
            ProgrammingLanguage::None => ProgrammingLanguageWithInfo::None,
            ProgrammingLanguage::Rust => {
                ProgrammingLanguageWithInfo::Rust(ProgrammingLanguageInfo {})
            }
            ProgrammingLanguage::Cpp => {
                ProgrammingLanguageWithInfo::Cpp(ProgrammingLanguageInfo {})
            }
        }
    }
}

const CPP_COMPILATION_FLAGS: [&str; 3] = [
    "main.cpp", // file to execute
    "-o",
    "executable", // executable name
];

const RUST_COMPILATION_FLAGS: [&str; 3] = [
    "main.rs", // file to execute
    "-o",
    "executable", // executable name
];
