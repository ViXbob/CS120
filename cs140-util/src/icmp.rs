use std::mem::MaybeUninit;
use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::time::Instant;
use std::net::{AddrParseError, IpAddr, Ipv4Addr, SocketAddrV4};
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use pnet::packet::icmp::echo_request::{MutableEchoRequestPacket, EchoRequestPacket, EchoRequest};
use pnet::packet::icmp::echo_reply::{EchoReply, EchoReplyPacket, MutableEchoReplyPacket};
use pnet::packet::icmp::{IcmpPacket, IcmpTypes, IcmpCode, checksum};
use pnet::packet::Packet;

use async_trait::async_trait;
use bitvec::bitarr;
use futures::select;
use log::trace;
use pnet::packet;
use smoltcp::wire::{Ipv4Packet, Icmpv4Packet, Icmpv4Message, Ipv4Address, IpProtocol};
use smoltcp::wire::ieee802154::Address;
use tokio::io;
use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use crate::rpc::{CS120RPC, CS120Socket, IcmpPackage, Transport};

pub struct IcmpSocket {
    send_input_sender: Sender<(Vec<u8>,SocketAddr)>,
    send_output_receiver: Receiver<io::Result<usize>>,
    recv_receiver:Receiver<io::Result<(usize, SocketAddr, Vec<u8>)>>,
}

impl IcmpSocket {
    pub fn new() -> Self {
        trace!("An icmp Socket instance created.");
        let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4)).expect("couldn't not create a icmp socket");
        // socket.set_read_timeout(Some(std::time::Duration::from_millis(100)));
        let socket_for_send_to = Arc::new(socket);
        let (send_input_sender, mut send_input_receiver) = channel::<(Vec<u8>, SocketAddr)>(1024);
        // let (send_notification_sender, mut send_notification_receiver) = channel(1024);
        let (send_output_sender,send_output_receiver) = channel(1024);
        let (recv_sender,recv_receiver) = channel(1024);

        let socket_for_recv = socket_for_send_to.clone();
        tokio::spawn(async move{
            loop{
                let data = send_input_receiver.recv().await;
                match data{
                    None => {return}
                    Some(data) => {
                        trace!("raw socket is ready to send a icmp request!!!!");
                        trace!("{:?}", data.0);
                        let addr: SockAddr = SockAddr::from(data.1);
                        let socket_for_send_to_cloned = socket_for_send_to.clone();
                        let result = tokio::task::spawn_blocking(move||{
                            socket_for_send_to_cloned.send_to(&data.0,&addr)
                        }).await.unwrap();
                        trace!("raw socket had sent a icmp request!!!!");
                        // send_notification_sender.send(true).await;
                        if send_output_sender.send(result).await.is_err(){
                            return;
                        }
                    }
                }
            }
        });

        tokio::spawn(async move{
            loop {
                let mut buf = Vec::with_capacity(128);
                buf.resize(128,MaybeUninit::new(0));
                // let socket_for_recv_cloned = socket_for_recv.clone();
                // trace!("ready to receive");
                // let (result,buf) = tokio::task::spawn_blocking(move ||{
                //     (socket_for_recv_cloned.recv_from(buf.as_mut_slice()),buf)
                // }).await.unwrap();
                let result = socket_for_recv.recv_from(buf.as_mut_slice());
                // trace!("reception complete!!!!!!!!!!!!!!!!!");
                let result = result.map(|(size,addr)|{
                    let buf = buf.into_iter().map(|u|{
                        unsafe{ u.assume_init()}
                    }).take(size).collect();
                    (size,addr.as_socket().unwrap(),buf)
                });
                if result.is_err() { continue; }
                trace!("{:?}",result);
                let sending_result = recv_sender.send(result).await;
                trace!("data sent");
                if sending_result.is_err() {
                    return;
                }
                trace!("received an icmp echo reply!");
            }
        });

        IcmpSocket {
            send_input_sender,
            send_output_receiver,
            recv_receiver,
        }
    }
    pub(crate) async fn send_to(&mut self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.send_input_sender.send((buf.iter().cloned().collect(),addr)).await;
        self.send_output_receiver.recv().await.unwrap()
    }
    pub(crate) async fn recv_from(&mut self) -> io::Result<(usize, SocketAddr, Vec<u8>)> {
        trace!("recv_from, before guard");
        // assert_eq!(self.debug_receiver.recv().await,Some(1));
        trace!("recv_from, after guard");
        let value = loop{
            match self.recv_receiver.try_recv(){
                Ok(value) => {
                    break value;
                }
                Err(_) => {
                    tokio::time::sleep(std::time::Duration::from_millis(1)).await;
                }
            }
        };
        trace!("value");
        value
    }
}

pub struct AudioPingUtil {
    send_ping_send: Sender<Ipv4Addr>,
    ping_result_recv: Receiver<(Ipv4Address, u128)>,
}

impl AudioPingUtil {
    pub fn new() -> Self {
        let layer = PhysicalLayer::new(1, 128);
        let layer = RedundancyLayer::new(layer);
        let mut layer = IPLayer::new(layer);
        let (send_ping_send, mut send_ping_recv) = channel::<Ipv4Addr>(1024);
        let (ping_result_send, ping_result_recv) = channel::<(Ipv4Address, u128)>(1024);
        let mut sequence_number: u16 = 0;
        let mut identifier: u16 = 0x02;
        tokio::spawn(async move {
            loop {
                let mut TIME = tokio::time::Instant::now();
                tokio::select! {
                    target = send_ping_recv.recv() => {
                        let target = target.unwrap();
                        let packet_size = EchoRequestPacket::minimum_packet_size();

                        let mut buf: Vec<u8> = vec![0; 20 + packet_size + 5];

                        let mut package = Ipv4Packet::new_unchecked(buf);
                        package.set_dst_addr(Ipv4Address::from(target));
                        package.set_protocol(IpProtocol::Icmp);
                        package.set_header_len(20);
                        package.set_total_len((20 + packet_size + 5).try_into().unwrap());
                        package.payload_mut()[8] = 0xff;
                        package.payload_mut()[9] = 0xff;
                        package.payload_mut()[10] = 0xff;
                        package.payload_mut()[11] = 0xff;
                        package.payload_mut()[12] = 0xff;
                        let mut icmp_package = Icmpv4Packet::new_unchecked(package.payload_mut());
                        icmp_package.set_echo_seq_no(sequence_number);
                        icmp_package.set_echo_ident(identifier);
                        icmp_package.set_msg_type(Icmpv4Message::EchoRequest);
                        icmp_package.set_msg_code(0);
                        icmp_package.fill_checksum();
                        package.fill_checksum();

                        TIME = tokio::time::Instant::now();

                        layer.send_package(package.into_inner()).await;

                        sequence_number += 1;
                    }
                    data = layer.recv_package() => {
                        let mut package = Ipv4Packet::new_unchecked(data);
                        let protocol = package.protocol();
                        let dst = package.src_addr();
                        let src = package.dst_addr();
                        match protocol {
                            IpProtocol::Icmp => {
                                let mut icmp_type: Icmpv4Message = Icmpv4Message::EchoRequest;
                                {
                                    let icmp_package = Icmpv4Packet::new_unchecked(package.payload_mut());
                                    icmp_type = icmp_package.msg_type();
                                }
                                match icmp_type {
                                    Icmpv4Message::EchoReply => {
                                        // println!("addr: {:?}", dst);
                                        let duration = TIME.elapsed();
                                        // println!("time={}ms", duration.as_millis());
                                        ping_result_send.send((dst, duration.as_millis())).await;
                                    }
                                    Icmpv4Message::EchoRequest => {
                                        package.set_dst_addr(dst);
                                        package.set_src_addr(src);
                                        let mut icmp_package = Icmpv4Packet::new_unchecked(package.payload_mut());
                                        icmp_package.set_msg_type(Icmpv4Message::EchoReply);
                                        icmp_package.set_msg_code(0);
                                        icmp_package.fill_checksum();
                                        package.fill_checksum();
                                        layer.send_package(package.into_inner()).await;
                                    }
                                    _ => {

                                    }
                                }
                            }
                            _ => {

                            }
                        }
                    }
                };
            }
        });
        AudioPingUtil{
            send_ping_send,
            ping_result_recv
        }
    }
    pub async fn ping_once(&mut self, target: Ipv4Addr) {
        self.send_ping_send.send(target).await;
        let (addr, time) = self.ping_result_recv.recv().await.unwrap();
        println!("addr: {:?}", addr);
        println!("time = {} ms", time);
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

    pub async fn ping_once(&mut self, target: Ipv4Addr) {
        let src = SocketAddr::from(SocketAddrV4::from_str("192.168.1.2:0").unwrap());
        let dst = SocketAddr::new(IpAddr::from(target), 0);
        let packet_size = EchoRequestPacket::minimum_packet_size();

        let mut buf: Vec<u8> = vec![0; 20 + packet_size];

        let mut package = Ipv4Packet::new_unchecked(buf);
        package.set_dst_addr(Ipv4Address::from(target));
        package.set_protocol(IpProtocol::Icmp);
        package.set_header_len(20);
        package.set_total_len((20 + packet_size).try_into().unwrap());
        let mut icmp_package = Icmpv4Packet::new_unchecked(package.payload_mut());
        icmp_package.set_echo_seq_no(self.sequence_number);
        icmp_package.set_echo_ident(self.identifier);
        icmp_package.set_msg_type(Icmpv4Message::EchoRequest);
        icmp_package.set_msg_code(0);
        icmp_package.fill_checksum();
        package.fill_checksum();

        println!("{:?}", package.payload_mut());

        let start_time = Instant::now();

        self.layer.send_package(package.into_inner()).await;

        self.sequence_number += 1;

        let mut data = self.layer.recv_package().await;
        let mut package = Ipv4Packet::new_unchecked(data);
        println!("receive a icmp package {:?}", package);
        let addr = package.src_addr();
        let protocol = package.protocol();
        let mut icmp_package = Icmpv4Packet::new_unchecked(package.payload_mut());
        let msg_type = icmp_package.msg_type();
        println!("receive a icmp package {:?}", icmp_package);


        match protocol {
            IpProtocol::Icmp => {
                match msg_type {
                    Icmpv4Message::EchoReply => {
                        println!("{:?}", icmp_package);
                        println!("addr: {:?}", addr);
                        let duration = start_time.elapsed();
                        println!("time={}ms", duration.as_millis());
                    }
                    _ => {

                    }
                }
            }
            _ => {

            }
        }
    }

    pub async fn wait_icmp_request_and_reply(&mut self) {
        let packet_size = EchoReplyPacket::minimum_packet_size();
        let mut buf: Vec<u8> = vec![0; packet_size + 32];
        loop {
            let mut data = self.layer.recv_package().await;
            let mut package = Ipv4Packet::new_unchecked(data);
            let dst = package.src_addr();
            let src = package.dst_addr();
            package.set_dst_addr(dst);
            package.set_src_addr(src);
            let mut icmp_package = Icmpv4Packet::new_unchecked(package.payload_mut());
            icmp_package.set_msg_type(Icmpv4Message::EchoReply);
            icmp_package.set_msg_code(0);
            icmp_package.fill_checksum();
            package.fill_checksum();
            self.layer.send_package(package.into_inner()).await;
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
        let mut sock = IcmpSocket::new();

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