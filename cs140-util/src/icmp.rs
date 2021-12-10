use std::mem::MaybeUninit;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::time::Instant;
use std::net::{AddrParseError, IpAddr};
use std::net::SocketAddr;
use std::str::FromStr;

use pnet::packet::icmp::echo_request::{MutableEchoRequestPacket, EchoRequestPacket};
use pnet::packet::icmp::echo_reply::EchoReplyPacket;
use pnet::packet::icmp::{IcmpPacket, IcmpTypes, IcmpCode, checksum};
use pnet::packet::Packet;

use async_trait::async_trait;
use tokio::io;

pub struct IcmpSocket {
    socket: Socket,
}

impl IcmpSocket {
    pub fn new() -> Self {
        let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4)).expect("couldn't not create a icmp socket");
        IcmpSocket {
            socket
        }
    }
    pub(crate) async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        todo!()
    }
    pub(crate) async fn recv_from(&self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> {
        todo!()
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

    pub fn run(&mut self, target: IpAddr, count: u32) {
        let maybe_sock = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4));

        if maybe_sock.is_err() {
            return;
        }

        let sock = maybe_sock.unwrap();

        let dst_addr = SockAddr::from(SocketAddr::new(target, 0));
        let packet_size = EchoRequestPacket::minimum_packet_size();

        let mut buf: Vec<u8> = vec![0; packet_size];
        for _ in 0..count {
            self.make_packet(&mut buf[..]);
            let start_time = Instant::now();
            sock.send_to(&buf[..], &dst_addr).unwrap();

            buf.clear();
            buf.resize(packet_size, 0);
            let mut buffer : Vec<MaybeUninit<u8>> = vec![MaybeUninit::new(0); 1024];

            let (len, address) = sock.recv_from(&mut buffer[..]).unwrap();

            let buf:Vec<_> = buffer.iter().take(len).map(|x|unsafe{ x.assume_init() }).collect();

            self.sequence_number += 1;

            if let Some(icmp_packet) = EchoReplyPacket::new(&buf[..]) {
                println!("{:?}", icmp_packet);
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
    #[test]
    fn ping_test() {
        let mut ping = Pinger::new(0x0001);
        ping.run(IpAddr::from_str("220.181.38.251").unwrap(), 20);
    }
}