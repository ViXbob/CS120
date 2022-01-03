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
use log::trace;
use tokio::io;
use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use crate::rpc::{CS120RPC, CS120Socket, IcmpPackage, Transport};

static TCPPORT: u16 = 33113;
static LOCALIPV4: Ipv4Addr = Ipv4Addr::new(10, 19, 75, 17);
static LOCALPORT: u16 = 11112;

pub struct TCPSocket {
    send_input_sender: Sender<(Vec<u8>,SocketAddr)>,
    send_output_receiver: Receiver<io::Result<usize>>,
    recv_receiver:Receiver<io::Result<(usize, SocketAddr, Vec<u8>)>>,
}

impl TCPSocket {
    pub fn new() -> Self {
        trace!("A TCPSocket instance created.");
        let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::TCP)).expect("couldn't not create a icmp socket");
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
                        trace!("raw socket is ready to send a tcp request!!!!");
                        let addr: SockAddr = SockAddr::from(data.1);
                        let socket_for_send_to_cloned = socket_for_send_to.clone();
                        println!("{:?}, {:?}", data.0, data.1);
                        let result = tokio::task::spawn_blocking(move||{
                            socket_for_send_to_cloned.send_to(&data.0, &addr)
                            // socket_for_send_to_cloned.send(&data.0)
                        }).await.unwrap();
                        println!("raw socket had sent a tcp request!!!!");
                        println!("len: {:?}", result);
                        trace!("raw socket had sent a tcp request!!!!");
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
                buf.resize(1024,MaybeUninit::new(0));
                // let socket_for_recv_cloned = socket_for_recv.clone();
                // trace!("ready to receive");
                // let (result,buf) = tokio::task::spawn_blocking(move ||{
                //     (socket_for_recv_cloned.recv_from(buf.as_mut_slice()),buf)
                // }).await.unwrap();
                let result = socket_for_recv.recv_from(buf.as_mut_slice());
                // trace!("reception complete!!!!!!!!!!!!!!!!!");
                let result = result.map(|(size,addr)|{
                    let buf : Vec<u8> = buf.into_iter().map(|u|{
                        unsafe{ u.assume_init()}
                    }).take(size).collect();
                    (size,addr.as_socket().unwrap(),buf)
                });
                if result.is_err() { continue; }
                trace!("{:?}",result);
                let buf = result.as_ref().unwrap().2.clone();
                let tcp_packet = pnet::packet::tcp::TcpPacket::new(&buf.as_slice()[20..]).unwrap();
                if tcp_packet.get_destination() != TCPPORT { continue; }
                let sending_result = recv_sender.send(result).await;
                trace!("data sent");
                if sending_result.is_err() {
                    return;
                }
                trace!("received an icmp echo reply!");
            }
        });

        TCPSocket {
            send_input_sender,
            send_output_receiver,
            recv_receiver,
        }
    }
    pub async fn send_to(&mut self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> {
        self.send_input_sender.send((buf.iter().cloned().collect(),addr)).await;
        self.send_output_receiver.recv().await.unwrap()
    }
    pub async fn recv_from(&mut self) -> io::Result<(usize, SocketAddr, Vec<u8>)> {
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

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn ping_test() {

    }
}