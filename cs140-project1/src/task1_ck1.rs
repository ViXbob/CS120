fn main() -> Result<(), anyhow::Error> {
    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");
    cs140_project1::record_wav::record(PATH, 10)?;
    Ok(())
}
