const PROTO_FILES: [&str; 10] = [
    "protos/common.proto",
    "protos/evaluation.proto",
    "protos/scoring.proto",
    "protos/worker.proto",
    "protos/service/contest.proto",
    "protos/service/dispatcher.proto",
    "protos/service/evaluation.proto",
    "protos/service/submission.proto",
    "protos/service/worker.proto",
    "protos/service/test.proto",
];

const ENUMS: [&str; 10] = [
    "common.ProgrammingLanguage",
    "evaluation.CompilationResult.Outcome",
    "evaluation.TestcaseResult.Outcome",
    "scoring.Subtask.Method",
    "scoring.Problem.Method",
    "scoring.User.Method.Aggregation",
    "worker.SourceFile.Type",
    "service.evaluation.EvaluationFile.Type",
    "service.evaluation.Problem.Type",
    "service.submission.SubmissionState",
];

const ENUM_ATTRIBUTES: &str =
    "#[derive(::strum_macros::EnumString,::strum_macros::ToString,::strum_macros::EnumIter)]";
const GENERAL_ATTRIBUTES: &str = "#[allow(unused_imports)]\nuse fake::*;\n#[derive(::serde::Serialize,::serde::Deserialize,::fake::Dummy)]"; // hacky hacks

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=build.rs");
    PROTO_FILES
        .iter()
        .for_each(|x| println!("cargo:rerun-if-changed={}", x));
    let mut config = prost_build::Config::new();
    config.type_attribute(".", &GENERAL_ATTRIBUTES);
    ENUMS.iter().for_each(|x| {
        config.type_attribute(x, &ENUM_ATTRIBUTES);
    });
    tonic_build::configure().compile_with_config(config, &PROTO_FILES, &["protos"])?;
    Ok(())
}
