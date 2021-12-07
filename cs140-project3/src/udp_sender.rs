use cs140_util::file_io::read_bytes_from_file;

const PATH: &str = "INPUT.txt";

fn main() {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("127.0.0.1:34242").expect("couldn't bind to address");
    let buf = read_bytes_from_file(PATH);
    socket.send_to(&buf.as_slice(), "127.0.0.1:34254").expect("couldn't send data");
}