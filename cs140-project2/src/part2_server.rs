use cs140_network::ack::state_machine::AckStateMachine;
use cs140_util::file_io::read_bytes_from_bin_file;

const PATH: &str = "C:\\Users\\Leomund\\Sources\\ShanghaiTech\\cs140\\cs140-project2\\OUTPUT.bin";
const SIZE: usize = 6250;

#[tokio::main]
fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let data = read_bytes_from_bin_file(PATH, SIZE);
    let mut server = AckStateMachine::new(0);
    server.append(data.iter().cloned());
    server.work().await;
}