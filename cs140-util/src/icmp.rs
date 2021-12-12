use std::mem::MaybeUninit;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::time::Instant;
use std::net::{AddrParseError, IpAddr, SocketAddrV4};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use pnet::packet::icmp::echo_request::{MutableEchoRequestPacket, EchoRequestPacket, EchoRequest};
use pnet::packet::icmp::echo_reply::{EchoReply, EchoReplyPacket, MutableEchoReplyPacket};
use pnet::packet::icmp::{IcmpPacket, IcmpTypes, IcmpCode, checksum};
use pnet::packet::Packet;

use async_trait::async_trait;
use tokio::io;
use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use crate::rpc::{CS120RPC, CS120Socket, IcmpPackage, Transport};

pub struct IcmpSocket {
    send_input_sender: Sender<(Vec<u8>,SocketAddr)>,
    send_output_receiver: tokio::sync::Mutex<Receiver<io::Result<usize>>>,
    recv_receiver: tokio::sync::Mutex<Receiver<io::Result<(usize, SocketAddr, Vec<u8>)>>>
}

impl IcmpSocket {
    pub fn new() -> Self {
        let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4)).expect("couldn't not create a icmp socket");
        // socket.set_read_timeout(Some(std::time::Duration::from_millis(100)));
        let socket_for_send_to = Arc::new(socket);
        let (send_input_sender, mut send_input_receiver) = channel::<(Vec<u8>, SocketAddr)>(1024);
        let (send_output_sender,send_output_receiver) = channel(1024);
        let (recv_sender,recv_receiver) = channel(1024);

        let socket_for_recv = socket_for_send_to.clone();
        tokio::spawn(async move{
            loop{
                let data = send_input_receiver.recv().await;
                match data{
                    None => {return}
                    Some(data) => {
                        let addr: SockAddr = SockAddr::from(data.1);
                        let result = socket_for_send_to.send_to(&data.0,&addr);
                        if send_output_sender.send(result).await.is_err(){
                            return;
                        }
                    }
                }
            }
        });

        tokio::spawn(async move{
            loop{
                let mut buf = Vec::with_capacity(128);
                buf.resize(128,MaybeUninit::new(0));
                let result = socket_for_recv.recv_from(buf.as_mut_slice());
                let result = result.map(|(size,addr)|{
                        let buf = buf.into_iter().map(|u|{
                            unsafe{ u.assume_init()}
                        }).take(size).collect();
                        (size,addr.as_socket().unwrap(),buf)
                });
                if recv_sender.send(result).await.is_err(){
                    return;
                }
            }
        });

        IcmpSocket {
            send_input_sender,
            send_output_receiver:tokio::sync::Mutex::new(send_output_receiver),
            recv_receiver: tokio::sync::Mutex::new(recv_receiver),
        }
    }
    pub(crate) async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.send_input_sender.send((buf.iter().cloned().collect(),addr)).await.unwrap();
        let mut guard = self.send_output_receiver.lock().await;
        guard.recv().await.unwrap()
    }
    pub(crate) async fn recv_from(&self) -> io::Result<(usize, SocketAddr, Vec<u8>)> {
        let mut guard = self.recv_receiver.lock().await;
        guard.recv().await.unwrap()
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
        let src = SocketAddr::from(SocketAddrV4::from_str("192.168.1.2:0").unwrap());
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

    pub async fn wait_icmp_request_and_reply(&mut self) {
        let packet_size = EchoReplyPacket::minimum_packet_size();
        let mut buf: Vec<u8> = vec![0; packet_size + 32];
        loop {
            if let CS120RPC::IcmpPackage(mut package) = self.layer.recv().await {
                let dst = package.src;
                let src = package.dst;
                self.make_reply_packet(package.data.as_mut_slice());
                self.layer.trans(CS120RPC::IcmpPackage(IcmpPackage{src, dst, types: IcmpTypes::EchoReply.0, data: package.data.clone()})).await;
            }
        }
    }

    fn make_reply_packet(&self, buf: &mut [u8]) {
        let mut echo_reply_packet = MutableEchoReplyPacket::new(buf).unwrap();
        echo_reply_packet.set_icmp_type(IcmpTypes::EchoReply);
        echo_reply_packet.set_icmp_code(IcmpCode::new(0));
        let echo_checksum = checksum(&IcmpPacket::new(echo_reply_packet.packet()).unwrap());
        echo_reply_packet.set_checksum(echo_checksum);
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