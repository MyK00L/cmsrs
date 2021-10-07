fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(".", "#[allow(unused_imports)]\nuse fake::*;\n#[derive(::serde::Serialize,::serde::Deserialize,::fake::Dummy)]") // hacky hacks
        .type_attribute("common.ProgrammingLanguage","#[derive(::strum_macros::EnumString,::strum_macros::ToString,::strum_macros::EnumIter)]")
        .type_attribute("common.CompilationResult.Outcome","#[derive(::strum_macros::EnumString,::strum_macros::ToString,::strum_macros::EnumIter)]")
        .type_attribute("scoring.Subtask.Method","#[derive(::strum_macros::EnumString,::strum_macros::ToString,::strum_macros::EnumIter)]")
        .type_attribute("scoring.Problem.Method","#[derive(::strum_macros::EnumString,::strum_macros::ToString,::strum_macros::EnumIter)]")
        .type_attribute("scoring.User.Method.Aggregation","#[derive(::strum_macros::EnumString,::strum_macros::ToString,::strum_macros::EnumIter)]")
        .type_attribute("worker.SourceFile.Type","#[derive(::strum_macros::EnumString,::strum_macros::ToString,::strum_macros::EnumIter)]")
        .type_attribute("service.evaluation.EvaluationFile.Type","#[derive(::strum_macros::EnumString,::strum_macros::ToString,::strum_macros::EnumIter)]")
        .type_attribute("service.evaluation.Problem.Type","#[derive(::strum_macros::EnumString,::strum_macros::ToString,::strum_macros::EnumIter)]")
        .type_attribute("service.submission.SubmissionState","#[derive(::strum_macros::EnumString,::strum_macros::ToString,::strum_macros::EnumIter)]")
        .type_attribute("common.Duration", "#[derive(::strum_macros::PartialOrd,::strum_macros::Ord)]")
        .compile(
            &[
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
            ],
            &["protos"],
        )?;
    Ok(())
}
