use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use socket2::SockAddr;
use tokio::net::UdpSocket;
use cs140_util::tcp::tcp::TCPSocket;
use cs140_util::rpc::{CS120RPC, TcpPackage};
use cs140_util::rpc::CS120Socket;

#[tokio::main]
async fn main() {
    // let mut tcp_socket = TCPSocket::new();
    // let buf: Vec<u8> = vec![43, 105, 0, 80, 253, 236, 248, 162, 0, 0, 0, 0, 128, 2, 0, 12, 218, 20, 0, 0, 2, 4, 0, 24, 3, 3, 0, 4, 2, 0, 0, 0];
    // let addr = Ipv4Addr::new(101, 32, 194, 18);
    // let addr = SocketAddrV4::new(addr, 80);
    // let addr = SocketAddr::from(addr);
    // // tcp_socket.send_to(&buf, addr).await;
    // loop {
    //     let mut buf: Vec<u8> = vec![0; 1024];
    //     let result = tcp_socket.recv_from().await;
    //     println!("result: {:?}", result.unwrap().2);
    // }

    let dst = SocketAddr::from(SocketAddrV4::from_str("10.19.75.4:34241").unwrap());
    let dst_addr = SocketAddr::from(SocketAddrV4::from_str("10.19.73.32:18888").unwrap());
    let mut tcp_socket = TCPSocket::new();
    let mut udp_socket = UdpSocket::bind("10.19.75.4:34241").await.unwrap();
    let mut buf = [0u8; 256];
    let mut udp_buf = [0u8; 1024];
    loop {
        tokio::select! {
            result = tcp_socket.recv_from_addr(&mut buf) => {
                if let Ok((len, addr)) = result {
                    let mut data: Vec<_> = buf.iter().take(len).map(|x| *x).collect();
                    let mut src;
                    let mut dst;
                    {
                        let mut ip_package = pnet::packet::ipv4::MutableIpv4Packet::new(&mut data).unwrap();
                        ip_package.set_destination(Ipv4Addr::new(10, 19, 75, 17));
                        ip_package.set_checksum(pnet::packet::ipv4::checksum(&ip_package.to_immutable()));
                        src = ip_package.get_source();
                        dst = ip_package.get_destination();
                        println!("{:?}", ip_package);
                    }
                    let mut data: Vec<u8> = data.iter_mut().map(|x| *x).collect();
                    let mut tcp_package = pnet::packet::tcp::MutableTcpPacket::new(&mut data[20..]).unwrap();
                    tcp_package.set_checksum(pnet::packet::tcp::ipv4_checksum(&tcp_package.to_immutable(), &src, &dst));
                    println!("send: {:?}", tcp_package);
                    udp_socket.send_to(data.as_slice(), dst_addr).await;
                }
            }
            result = udp_socket.recv_from_addr(&mut udp_buf) => {
                if let Ok((len, addr)) = result {
                    let mut data: Vec<_> = udp_buf.iter().take(len).map(|x| *x).collect();
                    let mut src;
                    let mut dst;
                    let mut dst_port;
                    {
                        let mut ip_package = pnet::packet::ipv4::MutableIpv4Packet::new(&mut data).unwrap();
                        ip_package.set_source(Ipv4Addr::new(10, 19, 75, 4));
                        ip_package.set_checksum(pnet::packet::ipv4::checksum(&ip_package.to_immutable()));
                        src = ip_package.get_source();
                        dst = ip_package.get_destination();
                        println!("{:?}", ip_package);
                    }
                    {
                        let mut tcp_package = pnet::packet::tcp::MutableTcpPacket::new(&mut data[20..]).unwrap();
                        tcp_package.set_checksum(pnet::packet::tcp::ipv4_checksum(&tcp_package.to_immutable(), &src, &dst));
                        println!("send icmp_packet: {:?}", tcp_package);
                        dst_port = tcp_package.get_destination();
                    }
                    // println!("{:?}", unsafe{String::from_utf8_unchecked(data.clone())});
                    tcp_socket.send_to_addr(&data.as_slice()[20..], SocketAddr::from(SocketAddrV4::new(dst, dst_port))).await;
                }
            }
        }
    }
}