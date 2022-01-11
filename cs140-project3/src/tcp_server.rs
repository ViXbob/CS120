use log::debug;
use smoltcp::socket::TcpSocket;
use smoltcp::time::Instant;
use cs140_util::file_io;
use cs140_util::tcp::athernet_tcp::AthernetTcpSocket;
use cs140_util::tcp::tcp_stack::TCPClient;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let data = file_io::read_bytes_from_file("INPUT.txt");
    let addr = std::net::Ipv4Addr::new(10, 20, 93, 103);
    let mut tcp_socket = AthernetTcpSocket::new(1);
    let src_port = 11113;
    tcp_socket.connect(addr, 18888, src_port).await;
    for pic in data.chunks(70) {
        // std::thread::sleep(std::time::Duration::from_millis(1000));
        tcp_socket.send(pic.to_vec(), src_port).await;
    }
    std::thread::park();
}