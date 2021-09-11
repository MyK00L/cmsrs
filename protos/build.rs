fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .type_attribute(".", "#[derive(::serde::Serialize,::serde::Deserialize,::fake::Dummy,::strum_macros::ToString,::strum_macros::EnumIter,::strum_macros::EnumString)]") // hacky hacks
        .type_attribute(".", "#[allow(unused_imports)]\nuse fake::*;") // more hacky hacks
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
