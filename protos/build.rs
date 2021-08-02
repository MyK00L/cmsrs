fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure().compile(
        &[
            "protos/common.proto",
            "protos/evaluation.proto",
            "protos/scoring.proto",
            "protos/user.proto",
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
    /*tonic_build::compile_protos("./protos/common.proto")?;
    tonic_build::compile_protos("./protos/scoring.proto")?;
    tonic_build::compile_protos("./protos/submission.proto")?;
    tonic_build::compile_protos("./protos/user.proto")?;
    tonic_build::compile_protos("./protos/worker.proto")?;

    tonic_build::compile_protos("./protos/service/contest.proto")?;
    tonic_build::compile_protos("./protos/service/dispatcher.proto")?;
    tonic_build::compile_protos("./protos/service/evaluation_files.proto")?;
    tonic_build::compile_protos("./protos/service/submission.proto")?;
    tonic_build::compile_protos("./protos/service/worker.proto")?;

    tonic_build::compile_protos("./protos/service/test.proto")?;*/
    Ok(())
}
