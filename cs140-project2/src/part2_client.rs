use cs140_network::ack::state_machine::AckStateMachine;
use cs140_util::file_io::write_bytes_into_bin_file;

// const PATH: &str = "/Users/vixbob/cs140/cs140-project2/OUTPUT.bin";
const PATH: &str = "C:\\Users\\Leomund\\Sources\\ShanghaiTech\\cs140\\cs140-project2\\OUTPUT.bin";
#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let mut client = AckStateMachine::new(0,2);
    client.work().await;
    write_bytes_into_bin_file(PATH, client.rx.as_slice());
}