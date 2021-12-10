use std::net::{SocketAddr, SocketAddrV4};
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use tokio::{
    net::{UdpSocket, TcpSocket},
    sync::mpsc::{channel, Receiver, Sender},
};
use cs140_network::encoding::HandlePackage;
use crate::rpc::{CS120RPC, Transport, CS120Socket, CS120ProtocolType, UdpPackage, IcmpPackage};
use cs140_network::ip::{IPLayer, IPPackage};
use async_trait::async_trait;
use log::trace;
use pnet::packet::icmp::{
    IcmpTypes::EchoReply,
    IcmpPacket,
};
use crate::icmp::IcmpSocket;

pub async fn run_nat(mut layer: IPLayer, listen_socket: impl CS120Socket + std::marker::Send + 'static, protocol_type: CS120ProtocolType) {
    let (audio_to_socket_sender, mut audio_to_socket_receiver) = channel::<CS120RPC>(1024);
    let (socket_to_audio_sender, mut socket_to_audio_receiver) = channel::<CS120RPC>(1024);
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
                                    let socket = UdpSocket::bind("10.19.73.32:23333").await.unwrap();
                                    let len = socket.send_to(package.data.as_slice(), package.dst).await.unwrap();
                                    trace!("send len: {}", len);
                                }
                                CS120RPC::IcmpPackage(package) => {
                                    let socket = IcmpSocket::new();
                                    let _ = socket.send_to(package.data.as_slice(), package.dst).await.unwrap();
                                }
                                CS120RPC::TcpPackage(package) => {

                                }
                            }
                        }
                    }
                }
                package = listen_socket.recv_from_addr(&mut buf) => {
                    match package {
                        Err(e) => {
                            return;
                        }
                        Ok((len, address)) => {
                            match protocol_type {
                                CS120ProtocolType::Udp => {
                                    trace!("received a socket package!");
                                    trace!("address: {:?}", address);
                                    let dst = SocketAddr::from(SocketAddrV4::from_str("192.168.1.2:0").unwrap());
                                    let data: Vec<u8> = buf.iter().take(len).map(|x| *x).collect();
                                    let package = CS120RPC::UdpPackage(UdpPackage{src: address, dst, data });
                                    socket_to_audio_sender.send(package).await;
                                }
                                CS120ProtocolType::Icmp => {
                                    let dst = SocketAddr::from(SocketAddrV4::from_str("192.168.1.2:0").unwrap());
                                    let data: Vec<u8> = buf.iter().take(len).map(|x| *x).collect();
                                    let icmp_packet = IcmpPacket::new(&data).unwrap();
                                    let package = CS120RPC::IcmpPackage(IcmpPackage{src: address, dst, types: icmp_packet.get_icmp_type().0, data});
                                    socket_to_audio_sender.send(package).await;
                                }
                                CS120ProtocolType::Tcp => {

                                }
                            };
                        }
                    }
                }
            }
        }
    });
}