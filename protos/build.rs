fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
    .type_attribute(".service.evaluation", "#[derive(::serde::Serialize,::serde::Deserialize)]")
    .type_attribute(".scoring", "#[derive(::serde::Serialize,::serde::Deserialize)]")
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
