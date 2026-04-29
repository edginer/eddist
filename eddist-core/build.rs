fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fds = protox::compile(["proto/bbs_events.proto"], ["proto/"])?;
    prost_build::Config::new().compile_fds(fds)?;
    Ok(())
}
