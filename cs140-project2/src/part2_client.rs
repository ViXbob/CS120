use cs140_network::ack::state_machine::AckStateMachine;
use cs140_util::file_io::write_bytes_into_bin_file;

const PATH: &str = "/Users/vixbob/cs140/cs140-project2/OUTPUT.bin";

fn main() {
    let mut client = AckStateMachine::new(&"USB Audio Device");
    client.work();
    write_bytes_into_bin_file(PATH, client.rx.as_slice());
}