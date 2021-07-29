fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("./protos/common.proto")?;
    tonic_build::compile_protos("./protos/contest_service.proto")?;
    tonic_build::compile_protos("./protos/dispatcher_service.proto")?;
    tonic_build::compile_protos("./protos/evaluation_files_service.proto")?;
    tonic_build::compile_protos("./protos/evaluation_info.proto")?;
    tonic_build::compile_protos("./protos/submission.proto")?;
    tonic_build::compile_protos("./protos/submission_service.proto")?;
    tonic_build::compile_protos("./protos/user_info.proto")?;
    tonic_build::compile_protos("./protos/worker_service.proto")?;
    Ok(())
}
