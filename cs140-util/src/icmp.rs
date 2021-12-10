use std::mem::MaybeUninit;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::time::Instant;
use std::net::{AddrParseError, IpAddr, SocketAddrV4};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use pnet::packet::icmp::echo_request::{MutableEchoRequestPacket, EchoRequestPacket};
use pnet::packet::icmp::echo_reply::EchoReplyPacket;
use pnet::packet::icmp::{IcmpPacket, IcmpTypes, IcmpCode, checksum};
use pnet::packet::Packet;

use async_trait::async_trait;
use tokio::io;
use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use crate::rpc::{CS120RPC, CS120Socket, IcmpPackage, Transport};

pub struct IcmpSocket {
    socket: Arc<Socket>,
}

impl IcmpSocket {
    pub fn new() -> Self {
        let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4)).expect("couldn't not create a icmp socket");
        // socket.set_read_timeout(Some(std::time::Duration::from_millis(100)));
        let socket = Arc::new(socket);
        IcmpSocket {
            socket
        }
    }
    pub(crate) async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        let buf = buf.to_vec();
        let addr: SockAddr = SockAddr::from(addr);
        let socket = self.socket.clone();
        tokio::spawn(async move {
            socket.send_to(&buf,&addr)
        }).await.unwrap()
    }
    pub(crate) async fn recv_from(&self) -> io::Result<(usize, SocketAddr, Vec<u8>)> {
        let mut buf = Vec::with_capacity(128);
        buf.resize(128,MaybeUninit::new(0));
        let socket = self.socket.clone();
        tokio::spawn(async move{
            let result = socket.recv_from(buf.as_mut_slice());
            result.map(|(size,addr)|{
                let buf = buf.into_iter().map(|u|{
                   unsafe{ u.assume_init()}
                }).take(size).collect();
                (size,addr.as_socket().unwrap(),buf)
            })
        }).await.unwrap()
    }
}

pub struct AudioPinger {
    layer: IPLayer,
    sequence_number: u16,
    identifier: u16
}

impl AudioPinger {
    pub fn new(layer: IPLayer, identifier: u16) -> Self {
        AudioPinger {
            layer,
            sequence_number: 0,
            identifier,
        }
    }

    pub async fn ping_once(&mut self, target: IpAddr) {
        let src = SocketAddr::from(SocketAddrV4::from_str("192.168.1.2:1234").unwrap());
        let dst = SocketAddr::new(target, 0);
        let packet_size = EchoRequestPacket::minimum_packet_size();

        let mut buf: Vec<u8> = vec![0; packet_size];
        self.make_packet(&mut buf[..]);
        let start_time = Instant::now();

        self.layer.trans(CS120RPC::IcmpPackage(IcmpPackage{src, dst, types: IcmpTypes::EchoRequest.0, data: buf})).await;

        self.sequence_number += 1;

        if let CS120RPC::IcmpPackage(package) = self.layer.recv().await {
            let buf = package.data.as_slice();
            let addr = package.src;
            if let Some(icmp_packet) = EchoReplyPacket::new(&buf[..]) {
                println!("{:?}", icmp_packet);
                println!("addr: {:?}", addr);
                let duration = start_time.elapsed();
                println!("time={}ms", duration.as_millis());
            }
        } else {
            unreachable!()
        }
    }

    fn make_packet(&self, buf: &mut [u8]) {
        let mut echo_packet = MutableEchoRequestPacket::new(buf).unwrap();
        echo_packet.set_sequence_number(self.sequence_number);
        echo_packet.set_identifier(self.identifier);
        echo_packet.set_icmp_type(IcmpTypes::EchoRequest);
        echo_packet.set_icmp_code(IcmpCode::new(0));

        let echo_checksum = checksum(&IcmpPacket::new(echo_packet.packet()).unwrap());
        echo_packet.set_checksum(echo_checksum);
    }
}

pub struct Pinger {
    sequence_number: u16,
    identifier: u16
}

impl Pinger {
    pub fn new(identifier: u16) -> Pinger {
        Pinger{
            sequence_number: 0,
            identifier,
        }
    }

    async fn run(&mut self, target: IpAddr, count: u32) {
        let sock = IcmpSocket::new();

        let dst_addr = SocketAddr::new(target, 0);

        let packet_size = EchoRequestPacket::minimum_packet_size();

        let mut buf: Vec<u8> = vec![0; packet_size];
        for _ in 0..count {
            self.make_packet(&mut buf[..]);
            let start_time = Instant::now();
            sock.send_to_addr(&buf[..], dst_addr).await.unwrap();

            buf.clear();
            buf.resize(packet_size, 0);
            let mut buffer : Vec<u8> = vec![0; 1024];

            let (len, address) = sock.recv_from_addr(&mut buffer[..]).await.unwrap();

            self.sequence_number += 1;

            if let Some(icmp_packet) = EchoReplyPacket::new(&buf[..]) {
                println!("{:?}", icmp_packet);
                println!("addr: {:?}", address);
                let duration = start_time.elapsed();
                println!("time={}ms", duration.as_millis());
            }
        }
    }

    fn make_packet(&self, buf: &mut [u8]) {
        let mut echo_packet = MutableEchoRequestPacket::new(buf).unwrap();
        echo_packet.set_sequence_number(self.sequence_number);
        echo_packet.set_identifier(self.identifier);
        echo_packet.set_icmp_type(IcmpTypes::EchoRequest);
        echo_packet.set_icmp_code(IcmpCode::new(0));

        let echo_checksum = checksum(&IcmpPacket::new(echo_packet.packet()).unwrap());
        echo_packet.set_checksum(echo_checksum);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn ping_test() {
        let mut ping = Pinger::new(0x0001);
        ping.run(IpAddr::from_str("101.32.194.18").unwrap(), 20).await;
    }
}