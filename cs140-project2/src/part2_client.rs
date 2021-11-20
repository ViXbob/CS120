use cs140_network::ack::state_machine::AckStateMachine;
use cs140_util::file_io::write_bytes_into_bin_file;

// const PATH: &str = "/Users/vixbob/cs140/cs140-project2/OUTPUT.bin";
const PATH: &str = "C:\\Users\\Leomund\\Sources\\ShanghaiTech\\cs140\\cs140-project2\\OUTPUT.bin";
#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let mut client = AckStateMachine::new(0,0,2);
    let begin_time = std::time::Instant::now();
    client.work().await;
    write_bytes_into_bin_file(PATH, client.rx.map(|x|x.unwrap()).concat().as_slice());
    let duration = begin_time.elapsed();
    println!("Transmission Complete!");
    println!("runtime is {}ms", duration.as_millis());
}