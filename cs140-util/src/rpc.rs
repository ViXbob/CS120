use std::fmt::Debug;
use std::net::{SocketAddr, SocketAddrV4};
use bincode::{config::Configuration, Decode, Encode};

use async_trait::async_trait;
use tokio::net::{TcpSocket, UdpSocket};
use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use crate::icmp::IcmpSocket;
use crate::tcp::tcp::TCPSocket;
use pnet::packet::icmp::IcmpType;
use pnet::packet::tcp;
use tokio::io;

#[async_trait]
pub trait Transport {
    type RPCTypeSet:Debug+Encode+Decode+PartialEq+Send;

    async fn send_package(&self,data: Vec<u8>);
    async fn recv_package(&self)->Vec<u8>;

    fn bincode_config(&self)->Configuration;

    async fn trans(&self,data: Self::RPCTypeSet){
        let encoded: Vec<u8> = bincode::encode_to_vec(&data,self.bincode_config()).unwrap();
        println!("encoded: {:?}",encoded);
        self.send_package(encoded).await;
    }

    async fn recv(&self)-> Self::RPCTypeSet{
        let data = self.recv_package().await;
        println!("received data: {:?}",data);
        let decoded = bincode::decode_from_slice(&data,self.bincode_config()).unwrap();
        println!("decoded: {:?}", decoded);
        return decoded;
    }
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub enum CS120RPC{
    IcmpPackage(IcmpPackage),
    UdpPackage(UdpPackage),
    TcpPackage(TcpPackage),
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct UdpPackage{
    pub src: SocketAddr,
    pub dst: SocketAddr,
    pub data: Vec<u8>,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct IcmpPackage{
    pub src: SocketAddr,
    pub dst: SocketAddr,
    pub types: u8,
    pub data: Vec<u8>,
}

#[derive(Encode, Decode, PartialEq, Debug, Clone)]
pub struct TcpPackage{
    pub src: SocketAddr,
    pub dst: SocketAddr,
    pub data: Vec<u8>,
}

pub enum CS120ProtocolType {
    Udp,
    Tcp,
    Icmp,
    IcmpEchoRequest,
}

#[async_trait]
impl Transport for IPLayer {
    type RPCTypeSet = CS120RPC;

    async fn send_package(&self, data: Vec<u8>) {
        // println!("length: {}, data: {:?}", data.len(), data);
        // let package = pnet::packet::ipv4::Ipv4Packet::new(data.as_slice());
        // let tcp_package = pnet::packet::tcp::TcpPacket::new(&data.as_slice()[20..]);
        // println!("ip_package: {:?}", package);
        // println!("tcp_package: {:?}", tcp_package);
        self.send(IPPackage::new(data)).await;
    }

    async fn recv_package(&self) -> Vec<u8> {
        self.receive().await.data
    }

    fn bincode_config(&self) -> Configuration {
        Configuration::standard()
    }
}

#[async_trait]
pub trait CS120Socket {
    async fn send_to_addr(&mut self, buf: &[u8], addr: SocketAddr) -> io::Result<usize> ;
    async fn recv_from_addr(&mut self, buf: &mut [u8]) -> io::Result<(usize, SocketAddr)> ;
}

#[async_trait]
impl CS120Socket for UdpSocket {
    async fn send_to_addr(&mut self, buf: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        self.send_to(buf, addr).await
    }

    async fn recv_from_addr(&mut self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        self.recv_from(buf).await
    }
}

#[async_trait]
impl CS120Socket for IcmpSocket {
    async fn send_to_addr(&mut self, buf: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        self.send_to(buf, addr).await
    }

    async fn recv_from_addr(&mut self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        let result = self.recv_from().await;
        match result {
            Ok((len, addr, buffer)) => {
                buf[..len].copy_from_slice(buffer.as_slice());
                Ok((len, addr))
            }
            Err(err) => {
                Err(err)
            }
        }
    }
}

#[async_trait]
impl CS120Socket for TCPSocket {
    async fn send_to_addr(&mut self, buf: &[u8], addr: SocketAddr) -> std::io::Result<usize> {
        self.send_to(buf, addr).await
    }

    async fn recv_from_addr(&mut self, buf: &mut [u8]) -> std::io::Result<(usize, SocketAddr)> {
        let result = self.recv_from().await;
        match result {
            Ok((len, addr, buffer)) => {
                buf[..len].copy_from_slice(buffer.as_slice());
                Ok((len, addr))
            }
            Err(err) => {
                Err(err)
            }
        }
    }
}

#[cfg(test)]
mod tests{

}