fn main() -> Result<(), anyhow::Error> {
    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");
    cs140_util::record::record(PATH, 10);
    Ok(())
}
