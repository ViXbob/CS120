use std::fs::File;
use std::io::BufReader;
use rodio::{Decoder, OutputStream, source::Source};

fn main() {
    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/music.mp3");
    println!("{}", PATH);
// Get a output stream handle to the default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
// Load a sound from a file, using a path relative to Cargo.toml
    let file = BufReader::new(File::open(PATH).unwrap());
// Decode that sound file into a source
    let source = Decoder::new(file).unwrap();
// Play the sound directly on the device
    stream_handle.play_raw(source.convert_samples());

// The sound plays in a separate audio thread,
// so we need to keep the main thread alive while it's playing.

    const PATH1: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded1.wav");
    let recorder = std::thread::spawn(|| cs140_project1::record_wav::record(PATH1, 10));
    recorder.join();
    // std::thread::sleep(std::time::Duration::from_secs(15));
    // drop(recoder);
}
