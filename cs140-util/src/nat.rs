use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use tokio::{
    net::{UdpSocket},
    sync::mpsc::{channel},
};
use cs140_network::encoding::HandlePackage;
use crate::rpc::{CS120RPC, Transport, CS120Socket, CS120ProtocolType, UdpPackage, IcmpPackage, TcpPackage};
use cs140_network::ip::{IPLayer};
use bincode::config::Configuration;
use log::trace;
use pnet::packet::icmp::{
    IcmpPacket,
};
use crate::icmp::IcmpSocket;
use crate::tcp::tcp::TCPSocket;

static TCPPORT: u16 = 33113;

pub async fn run_nat(mut layer: IPLayer, mut listen_socket: impl CS120Socket + std::marker::Send + 'static, protocol_type: CS120ProtocolType) {
    let (audio_to_socket_sender, mut audio_to_socket_receiver) = channel::<CS120RPC>(1024);
    let (socket_to_audio_sender, mut socket_to_audio_receiver) = channel::<CS120RPC>(1024);
    let mut icmp_socket = IcmpSocket::new();
    let mut tcp_socket = TCPSocket::new();
    tokio::spawn(async move {
        loop {
            tokio::select! {
                package = socket_to_audio_receiver.recv() => {
                    match package {
                        None => {
                            return;
                        }
                        Some(package) => {
                            trace!("received a socket package!");
                            layer.trans(package).await;
                        }
                    }
                }
                package = layer.recv() => {
                    audio_to_socket_sender.send(package).await;
                }
            }
        }
    });
    tokio::spawn(async move {
        let mut buf = vec![0u8; 10240];
        loop {
            tokio::select! {
                package = audio_to_socket_receiver.recv() => {
                    match package {
                        None => {
                            return;
                        }
                        Some(package) => {
                            trace!("received a CS120RPC package!");
                            match package {
                                CS120RPC::UdpPackage(package) => {
                                    trace!("{:?}, {:?}", package.src, package.dst);
                                    trace!("ok!");
                                    let mut socket = UdpSocket::bind("10.19.73.32:23333").await.unwrap();
                                    let len = socket.send_to_addr(package.data.as_slice(), package.dst).await.unwrap();
                                    trace!("send len: {}", len);
                                }
                                CS120RPC::IcmpPackage(package) => {
                                    match protocol_type {
                                        CS120ProtocolType::Icmp => {
                                            // let socket = IcmpSocket::new();
                                            let _ = icmp_socket.send_to_addr(package.data.as_slice(), package.dst).await.unwrap();
                                            let mut buffer : Vec<u8> = vec![0; 128];
                                            trace!("send an icmp echo request");
                                            let result = icmp_socket.recv_from_addr(buffer.as_mut_slice()).await;
                                            trace!("receive an icmp echo reply");
                                            match result {
                                                Ok((len, address)) => {
                                                    let dst = SocketAddr::from(SocketAddrV4::from_str("192.168.1.2:0").unwrap());
                                                    let data: Vec<u8> = buf.iter().take(len).map(|x| *x).collect();
                                                    let icmp_packet = IcmpPacket::new(&data).unwrap();
                                                    let package = CS120RPC::IcmpPackage(IcmpPackage{src: address, dst, types: icmp_packet.get_icmp_type().0, data});
                                                    trace!("ready to send socket package to audio");
                                                    socket_to_audio_sender.send(package).await;
                                                    trace!("send socket package to audio");
                                                }
                                                Err(e) => {
                                                }
                                            }
                                        }
                                        CS120ProtocolType::IcmpEchoRequest => {
                                            println!("icmp echo reply: {:?} to {:?}, and next_hop: {:?}", package, package.dst, package.src);
                                            let dst = package.src;
                                            let encoded: Vec<u8> = bincode::encode_to_vec(CS120RPC::IcmpPackage(package), Configuration::standard()).unwrap();
                                            let mut socket = UdpSocket::bind("10.19.73.32:22791").await.unwrap();
                                            socket.send_to_addr(encoded.as_slice(), dst).await;
                                        }
                                        _ => {
                                            unreachable!()
                                        }
                                    }
                                }
                                CS120RPC::TcpPackage(package) => {
                                    let mut data = package.data.clone();
                                    // let mut src;
                                    // let mut dst;
                                    // {
                                    //     let mut ip_package = pnet::packet::ipv4::MutableIpv4Packet::new(&mut data).unwrap();
                                    //     // ip_package.set_source(Ipv4Addr::new(10, 19, 73, 32));
                                    //     // ip_package.set_checksum(pnet::packet::ipv4::checksum(&ip_package.to_immutable()));
                                    //     src = ip_package.get_source();
                                    //     dst = ip_package.get_destination();
                                    //     println!("{:?}", ip_package);
                                    // }
                                    // let mut data: Vec<u8> = data.iter_mut().map(|x| *x).collect();
                                    // let mut tcp_package = pnet::packet::tcp::MutableTcpPacket::new(&mut data[20..]).unwrap();
                                    // // tcp_package.set_source(TCPPORT);
                                    // // tcp_package.set_checksum(pnet::packet::tcp::ipv4_checksum(&tcp_package.to_immutable(), &src, &dst));
                                    // println!("{}, {}", tcp_package.get_checksum(), pnet::packet::tcp::ipv4_checksum(&tcp_package.to_immutable(), &src, &dst));
                                    // println!("{:?}, {:?}", tcp_package, package.dst);
                                    // // let DATA: Vec<u8> = vec![8, 90, 0, 80, 52, 68, 189, 120, 109, 10, 154, 155, 80, 16, 4, 3, 47, 94, 0, 0, 0];
                                    // // b"08 5a 00 50 34 44 bd 78 6d 0a 9a 9b 50 10 04 03 2f 5e 00 00"
                                    // let _ = tcp_socket.send_to_addr(&data[20..], package.dst).await.unwrap();
                                    let mut socket = UdpSocket::bind("10.19.73.32:22791").await.unwrap();
                                    println!("send an tcp package!!!");
                                    let tcp_package = pnet::packet::tcp::TcpPacket::new(&data[20..]);
                                    println!("tcp package contain {:?}", tcp_package);
                                    socket.send_to_addr(package.data.as_slice(), SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(10, 19, 75, 4), 34241))).await;
                                }
                            }
                        }
                    }
                }
                package = listen_socket.recv_from_addr(&mut buf) => {
                    match package {
                        Err(e) => {
                            // println!("{:?}", e);
                            // return;
                        }
                        Ok((len, address)) => {
                            println!("receive package!");
                            match protocol_type {
                                CS120ProtocolType::Udp => {
                                    trace!("received a socket package!");
                                    trace!("address: {:?}", address);
                                    let dst = SocketAddr::from(SocketAddrV4::from_str("192.168.1.2:0").unwrap());
                                    let data: Vec<u8> = buf.iter().take(len).map(|x| *x).collect();
                                    let package = CS120RPC::UdpPackage(UdpPackage{src: address, dst, data });
                                    socket_to_audio_sender.send(package).await;
                                }
                                CS120ProtocolType::IcmpEchoRequest => {
                                    let data = &buf.clone().to_vec()[..len];
                                    let decoded: CS120RPC = bincode::decode_from_slice(data, Configuration::standard()).unwrap();
                                    socket_to_audio_sender.send(decoded).await;
                                    trace!("send!");
                                }
                                CS120ProtocolType::Icmp => {
                                    let dst = SocketAddr::from(SocketAddrV4::from_str("192.168.1.2:0").unwrap());
                                    let data: Vec<u8> = buf.iter().take(len).map(|x| *x).collect();
                                    let icmp_packet = IcmpPacket::new(&data).unwrap();
                                    let package = CS120RPC::IcmpPackage(IcmpPackage{src: address, dst, types: icmp_packet.get_icmp_type().0, data});
                                    socket_to_audio_sender.send(package).await;
                                }
                                CS120ProtocolType::Tcp => {
                                    let dst = SocketAddr::from(SocketAddrV4::from_str("10.19.75.17:11113").unwrap());
                                    let data: Vec<u8> = buf.iter().take(len).map(|x| *x).collect();
                                    let package = CS120RPC::TcpPackage(TcpPackage{src: address, dst, data });
                                    socket_to_audio_sender.send(package).await;
                                }
                            };
                        }
                    }
                }
            }
        }
    });
}