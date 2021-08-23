fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile(
        &[
            "protos/common.proto",
            "protos/evaluation.proto",
            "protos/scoring.proto",
            "protos/worker.proto",
            "protos/service/contest.proto",
            "protos/service/dispatcher.proto",
            "protos/service/evaluation_files.proto",
            "protos/service/submission.proto",
            "protos/service/worker.proto",
            "protos/service/test.proto",
        ],
        &["protos"],
    )?;
    Ok(())
}
