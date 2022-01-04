use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use socket2::SockAddr;
use cs140_util::tcp::tcp::TCPSocket;

#[tokio::main]
async fn main() {
    let mut tcp_socket = TCPSocket::new();
    let buf: Vec<u8> = vec![43, 105, 0, 80, 253, 236, 248, 162, 0, 0, 0, 0, 128, 2, 0, 12, 218, 20, 0, 0, 2, 4, 0, 24, 3, 3, 0, 4, 2, 0, 0, 0]
    let addr = Ipv4Addr::new(101, 32, 194, 18);
    let addr = SocketAddrV4::new(addr, 80);
    let addr = SocketAddr::from(addr);
    tcp_socket.send_to(&buf, addr).await;
}