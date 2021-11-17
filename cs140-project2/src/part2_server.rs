use cs140_network::ack::state_machine::AckStateMachine;
use cs140_util::file_io::read_bytes_from_bin_file;

const PATH: &str = "/Users/vixbob/cs140/cs140-project2/INPUT.bin";
const SIZE: usize = 6250;

fn main() {
    let data = read_bytes_from_bin_file(PATH, SIZE);
    let mut server = AckStateMachine::new(&"USB Audio Device");
    server.append(data.iter().cloned());
    server.work();
}