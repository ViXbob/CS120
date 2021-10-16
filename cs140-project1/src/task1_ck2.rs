use rodio::{source::Source, Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;

fn main() -> Result<(), anyhow::Error> {
    // const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/music.mp3");
    const PATH: &str = "cs140-playground/music.mp3";
    println!("{}", PATH);
    // Get a output stream handle to the default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default()?;
    // Load a sound from a file, using a path relative to Cargo.toml
    let file = BufReader::new(File::open(PATH)?);
    // Decode that sound file into a source
    let source = Decoder::new(file)?;
    // Play the sound directly on the device
    stream_handle.play_raw(source.convert_samples())?;

    // The sound plays in a separate audio thread,
    // so we need to keep the main thread alive while it's playing.

    // const PATH1: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded1.wav");
    const PATH1: &str = "cs140-playground/recorded1.wav";
    cs140_util::record::record(PATH1, 10);
    Ok(())
}
