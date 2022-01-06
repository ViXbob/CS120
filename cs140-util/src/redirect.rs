use std::{
    net::{
        IpAddr,
        Ipv4Addr,
        SocketAddr,
    },
};
use tokio::{
    net::{
        UdpSocket,
    }
};
use crate::{
    icmp::IcmpSocket,
    tcp::tcp::TCPSocket,
    rpc::CS120Socket,
};
use smoltcp::{
    wire::{
        Ipv4Packet,
        Ipv4Address,
        IpAddress,
        IpProtocol,
        TcpPacket,
    },
};
use log::{trace, debug, info, warn};

static PORT: u16 = crate::new_nat::PORT;
static ADDR: Ipv4Addr = Ipv4Addr::new(10, 19, 75, 17);
type IpPacket = Ipv4Packet<Vec<u8>>;

pub async fn run_unix_redirect_server(local_addr: Ipv4Addr, nat_server_addr: Ipv4Addr) {
    let mut udp_socket = UdpSocket::bind(SocketAddr::new(IpAddr::from(local_addr), PORT)).await.unwrap();
    let nat_server_addr = SocketAddr::new(IpAddr::from(nat_server_addr), PORT);
    let mut icmp_socket = IcmpSocket::new();
    let mut tcp_socket = TCPSocket::new();
    let mut icmp_buf: Vec<u8> = vec![0u8; 1024000];
    let mut tcp_buf: Vec<u8> = vec![0u8; 1024000];
    let mut udp_buf: Vec<u8> = vec![0u8; 1024000];
    loop{
        tokio::select! {
            result = icmp_socket.recv_from_addr(&mut icmp_buf) => {
                if let Ok((len, addr)) = result {
                    let mut data:Vec<u8> = icmp_buf.iter().take(len).map(|x| *x).collect();
                    let mut package = Ipv4Packet::new_unchecked(data);
                    package.set_dst_addr(Ipv4Address::from(ADDR));
                    trace!("receive a icmp package: {:?}", package);
                    udp_socket.send_to(package.into_inner().as_slice(), nat_server_addr).await;
                }
            }
            result = tcp_socket.recv_from_addr(&mut tcp_buf) => {
                if let Ok((len, addr)) = result {
                    let mut data:Vec<u8> = tcp_buf.iter().take(len).map(|x| *x).collect();
                    let mut package = Ipv4Packet::new_unchecked(data);
                    package.set_dst_addr(Ipv4Address::from(ADDR));
                    let src = package.src_addr();
                    let dst = package.dst_addr();
                    trace!("ip header length: {}, ip packet length: {}", package.header_len(), package.total_len());
                    let mut tcp_package = TcpPacket::new_unchecked(package.payload_mut());
                    trace!("tcp packet: {:?}", tcp_package);
                    tcp_package.fill_checksum(&IpAddress::from(src), &IpAddress::from(dst));
                    package.fill_checksum();
                    trace!("receive a tcp package: {:?}", package);
                    udp_socket.send_to(package.into_inner().as_slice(), nat_server_addr).await;
                }
            }
            result = udp_socket.recv_from_addr(&mut udp_buf) => {
                if let Ok((len, addr)) = result {
                    let mut data: Vec<u8> = udp_buf.iter().take(len).map(|x| *x).collect();
                    let mut package = Ipv4Packet::new_unchecked(data);
                    let src = Ipv4Address::from(local_addr);
                    let dst = package.dst_addr();
                    match package.protocol() {
                        IpProtocol::Icmp => {
                            trace!("send a icmp package: {:?}", package);
                            icmp_socket.send_to_addr(package.payload_mut(), SocketAddr::new(IpAddr::from(Ipv4Addr::from(dst)), 0)).await;
                        }
                        IpProtocol::Tcp => {
                            trace!("send a tcp package: {:?}", package);
                            let mut tcp_package = TcpPacket::new_unchecked(package.payload_mut());
                            let dst_port = tcp_package.dst_port();
                            tcp_package.fill_checksum(&IpAddress::from(src), &IpAddress::from(dst));
                            tcp_socket.send_to_addr(tcp_package.into_inner(), SocketAddr::new(IpAddr::from(Ipv4Addr::from(dst)), dst_port)).await;
                        }
                        _ => {

                        }
                    }
                }
            }
        }
    };
}