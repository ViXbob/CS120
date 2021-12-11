use std::net::{SocketAddr, SocketAddrV4};
use std::str::FromStr;
use bincode::config::Configuration;
use tokio::net::UdpSocket;
use cs140_util::icmp::IcmpSocket;
use cs140_util::rpc::{CS120RPC, CS120Socket, IcmpPackage};
use pnet::packet::icmp::echo_reply::EchoReplyPacket;
use pnet::packet::icmp::IcmpTypes;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};

#[tokio::main]
async fn main() {
    let dst = SocketAddr::from(SocketAddrV4::from_str("192.168.1.2:0").unwrap());
    let dst_addr = SocketAddr::from(SocketAddrV4::from_str("10.19.73.32:18888").unwrap());
    let icmp_socket = IcmpSocket::new();
    let udp_socket = UdpSocket::bind("10.19.75.4:34241").await.unwrap();
    let mut buf = [0u8; 256];
    loop {
        let result = icmp_socket.recv_from_addr(&mut buf).await;
        let data = buf.clone();
        if let Ok((len, addr)) = result {
            let icmp_package = CS120RPC::IcmpPackage(IcmpPackage{src: addr, dst, types: IcmpTypes::EchoRequest.0, data: Vec::from(&data[..len]) });
            println!("send: {:?}", icmp_package);
            let encoded: Vec<u8> = bincode::encode_to_vec(icmp_package,Configuration::standard()).unwrap();
            udp_socket.send_to(encoded.as_slice(), dst_addr).await;
        }
    }
}