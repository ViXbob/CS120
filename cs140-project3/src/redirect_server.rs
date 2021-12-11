use std::net::{SocketAddr, SocketAddrV4};
use std::str::FromStr;
use bincode::config::Configuration;
use tokio::net::UdpSocket;
use cs140_util::icmp::IcmpSocket;
use cs140_util::rpc::{CS120RPC, CS120Socket, IcmpPackage};
use pnet::packet::icmp::echo_reply::EchoReplyPacket;
use pnet::packet::icmp::{IcmpTypes, MutableIcmpPacket, IcmpCode, checksum, IcmpPacket};
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use pnet::packet::icmp::echo_request::{MutableEchoRequestPacket, EchoRequestPacket, EchoRequest};
use pnet::packet::icmp::echo_reply::{EchoReply, MutableEchoReplyPacket};
use pnet::packet::Packet;

#[tokio::main]
async fn main() {
    let dst = SocketAddr::from(SocketAddrV4::from_str("10.19.75.4:34241").unwrap());
    let dst_addr = SocketAddr::from(SocketAddrV4::from_str("10.19.73.32:18888").unwrap());
    let icmp_socket = IcmpSocket::new();
    let udp_socket = UdpSocket::bind("10.19.75.4:34241").await.unwrap();
    let mut buf = [0u8; 256];
    let mut udp_buf = [0u8; 1024];
    loop {
        tokio::select! {
            result = icmp_socket.recv_from_addr(&mut buf) => {
                if let Ok((len, addr)) = result {
                    let mut data: Vec<_> = buf.iter().skip(20).take(len - 20).map(|x| *x).collect();
                    let icmp_packet = MutableIcmpPacket::new(data.as_mut_slice()).unwrap();
                    println!("icmp_packet: {:?}", icmp_packet);
                    if icmp_packet.get_icmp_type() == IcmpTypes::EchoRequest {
                        let icmp_package = CS120RPC::IcmpPackage(IcmpPackage { src: addr, dst, types: IcmpTypes::EchoRequest.0, data });
                        println!("send: {:?}", icmp_package);
                        let encoded: Vec<u8> = bincode::encode_to_vec(icmp_package, Configuration::standard()).unwrap();
                        // udp_socket.send_to(encoded.as_slice(), dst_addr).await;
                        let mut icmp_buf = [0u8; 128];
                        let mut echo_reply_packet = MutableEchoReplyPacket::new(&mut icmp_buf).unwrap();
                        echo_reply_packet.set_sequence_number(0);
                        echo_reply_packet.set_identifier(0x0002);
                        echo_reply_packet.set_icmp_type(IcmpTypes::EchoReply);
                        echo_reply_packet.set_icmp_code(IcmpCode::new(0));
                        echo_reply_packet.set_payload(b"echo_reply_from_cs120_athernet");
                        let echo_checksum = checksum(&IcmpPacket::new(echo_reply_packet.packet()).unwrap());
                        echo_reply_packet.set_checksum(echo_checksum);
                        icmp_socket.send_to_addr(&icmp_buf, addr);
                    }
                }
            }
            result = udp_socket.recv_from_addr(&mut udp_buf) => {
                if let Ok((len, addr)) = result {
                    let mut data: Vec<_> = udp_buf.iter().take(len).map(|x| *x).collect();
                    let mut decoded: CS120RPC = bincode::decode_from_slice(data.as_slice(), Configuration::standard()).unwrap();
                    if let CS120RPC::IcmpPackage(package) = decoded {
                        println!("send icmp_packet: {:?}", package);
                        println!("{:?}", unsafe{String::from_utf8_unchecked(package.data.clone())});
                        icmp_socket.send_to_addr(package.data.as_slice(), package.dst).await;
                    }
                }
            }
        }
    }
}