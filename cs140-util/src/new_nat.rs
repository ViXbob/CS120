use std::{
    net::{
        SocketAddr,
        Ipv4Addr,
        IpAddr,
    },
    sync::Arc,
};
use tokio::{
    net::{
        UdpSocket,
        ToSocketAddrs,
    },
    sync::{
        mpsc::channel,
        Mutex,
    },
};
use cs140_network::{
    ip::IPLayer,
    physical::PhysicalLayer,
    redundancy::RedundancyLayer,
};
use smoltcp::{
    wire::Ipv4Packet,
};
use log::{
    trace,
    info,
    warn,
    debug,
};
use crate::rpc::Transport;

pub static PORT: u16 = 18888;

pub async fn run_nat_server(local_addr: Ipv4Addr, unix_server_addr: Ipv4Addr) {
    let udp_socket = UdpSocket::bind(SocketAddr::new(IpAddr::from(local_addr), PORT)).await.unwrap();
    let unix_server_addr = SocketAddr::new(IpAddr::from(unix_server_addr), PORT);
    let layer = PhysicalLayer::new(1, 128);
    let layer = RedundancyLayer::new(layer);
    let layer = IPLayer::new(layer);
    let (audio_to_socket_sender, mut audio_to_socket_receiver) = channel::<Ipv4Packet<Vec<u8>>>(1024);
    let (socket_to_audio_sender, mut socket_to_audio_receiver) = channel::<Ipv4Packet<Vec<u8>>>(1024);
    tokio::spawn(async move {
        loop {
            tokio::select! {
                    package = socket_to_audio_receiver.recv() => {
                        match package {
                            None => {
                                return;
                            }
                            Some(package) => {
                                trace!("a package is about ot send to audio server: {:?}", package);
                                layer.send_package(package.into_inner()).await;
                            }
                        }
                    }
                    package = layer.recv_package() => {
                        audio_to_socket_sender.send(Ipv4Packet::new_unchecked(package)).await;
                    }
                }
        }
    });
    tokio::spawn(async move {
        let mut buf = vec![0u8; 1024000];
        loop {
            tokio::select! {
                package = audio_to_socket_receiver.recv() => {
                    match package {
                        None => {
                            return;
                        }
                        Some(package) => {
                            trace!("a package is about to send to unix server, {:?}", package);
                            let result = udp_socket.send_to::<SocketAddr>(package.into_inner().as_slice(), unix_server_addr).await;
                            if result.is_err() {
                                warn!("UdpSocket failed to send a package!");
                            }
                        }
                    }
                }
                len = udp_socket.recv(buf.as_mut_slice()) => {
                    if len.is_err() {
                        warn!("UdpSocket failed to receive a package!");
                    }
                    let len = len.ok().unwrap();
                    let data: Vec<u8> = buf.iter().take(len).map(|x| *x).collect();
                    socket_to_audio_sender.send(Ipv4Packet::new_unchecked(data)).await;
                }
            }
        }
    });
}