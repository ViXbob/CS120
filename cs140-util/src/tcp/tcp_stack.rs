use std::collections::BTreeMap;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use smoltcp::iface::{InterfaceBuilder, NeighborCache, SocketHandle, Routes, Interface};
use smoltcp::phy::{Device, Medium, FaultInjector, Tracer, PcapWriter, PcapMode};
use smoltcp::socket::{TcpSocket, TcpSocketBuffer};
use smoltcp::socket::{UdpPacketMetadata, UdpSocket, UdpSocketBuffer};
use smoltcp::time::{Duration, Instant};
use smoltcp::wire::{EthernetAddress, IpAddress, IpCidr, Ipv4Address, TcpRepr, IpRepr};
use crate::rpc::CS120RPC::TcpPackage;
use crate::tcp::athernet_interface::AthernetInterface;

pub struct TCPClient<'a> {
    pub tcp_handle: SocketHandle,
    pub iface: Interface<'a, FaultInjector<Tracer<PcapWriter<AthernetInterface, Box<dyn Write>>>>>,
}

impl TCPClient<'_> {
    pub fn new(mtu: usize) -> Self {
        let device = AthernetInterface::new(mtu, Medium::Ip);

        let device = middleware(device, /*loopback=*/ false);

        let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 64]);
        let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 128]);
        let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);

        let ip_addrs = [IpCidr::new(IpAddress::v4(10, 19, 75, 17), 24)];
        let default_v4_gw = Ipv4Address::new(192, 168, 69, 100);
        let mut routes = Routes::new(BTreeMap::new());
        routes.add_default_ipv4_route(default_v4_gw).unwrap();

        let mut builder = InterfaceBuilder::new(device, vec![])
            .ip_addrs(ip_addrs)
            .routes(routes);
        let mut iface = builder.finalize();
        let tcp_handle = iface.add_socket(tcp_socket);
        TCPClient {
            tcp_handle,
            iface
        }
    }

    pub fn connect(&mut self, dst: std::net::Ipv4Addr, dst_port: u16, local_port: u16) {
        let (socket, cx) = self.iface.get_socket_and_context::<TcpSocket>(self.tcp_handle);
        socket.connect(cx, (dst, dst_port), local_port).unwrap();
    }

    pub fn send(&mut self, buf: &[u8]) -> smoltcp::Result<usize> {
        let socket = self.iface.get_socket::<TcpSocket>(self.tcp_handle);
        println!("{}", socket.is_active());
        socket.send_slice(buf)
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> smoltcp::Result<usize> {
        let socket = self.iface.get_socket::<TcpSocket>(self.tcp_handle);
        socket.recv_slice(buf)
    }
}

pub struct TCPServer {
    tcp_handle: SocketHandle,
}

impl TCPServer {
    pub fn new(mtu: usize) -> Self {
        let device = AthernetInterface::new(mtu, Medium::Ip);

        let device = middleware(device, /*loopback=*/ false);

        let tcp_rx_buffer = TcpSocketBuffer::new(vec![0; 64]);
        let tcp_tx_buffer = TcpSocketBuffer::new(vec![0; 128]);
        let tcp_socket = TcpSocket::new(tcp_rx_buffer, tcp_tx_buffer);

        let ip_addrs = [IpCidr::new(IpAddress::v4(192, 168, 69, 2), 24)];

        let medium = device.capabilities().medium;
        let mut builder = InterfaceBuilder::new(device, vec![]).ip_addrs(ip_addrs);
        let mut iface = builder.finalize();
        let tcp_handle = iface.add_socket(tcp_socket);
        TCPServer {
            tcp_handle
        }
    }
}

fn middleware<D>(
    device: D,
    loopback: bool,
) -> FaultInjector<Tracer<PcapWriter<D, Box<dyn io::Write>>>>
    where
        D: for<'a> Device<'a>,
{
    let drop_chance = 0;
    let corrupt_chance = 0;
    let size_limit = 0;
    let tx_rate_limit = 0;
    let rx_rate_limit = 0;
    let shaping_interval = 0;

    let pcap_writer: Box<dyn io::Write> = Box::new(io::sink());

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();

    let device = PcapWriter::new(
        device,
        pcap_writer,
        if loopback {
            PcapMode::TxOnly
        } else {
            PcapMode::Both
        },
    );

    let device = Tracer::new(device, |_timestamp, _printer| {
        #[cfg(feature = "log")]
        trace!("{}", _printer);
    });

    let mut device = FaultInjector::new(device, seed);
    device.set_drop_chance(drop_chance);
    device.set_corrupt_chance(corrupt_chance);
    device.set_max_packet_size(size_limit);
    device.set_max_tx_rate(tx_rate_limit);
    device.set_max_rx_rate(rx_rate_limit);
    device.set_bucket_interval(Duration::from_millis(shaping_interval));
    device
}

#[cfg(test)]
mod tests{
    use super::*;
    #[tokio::test]
    async fn test_tcp_client(){
        let mtu: usize = 64;
        let addr = std::net::Ipv4Addr::new(101, 32, 194, 18);
        let mut tcp_client = TCPClient::new(mtu);
        tcp_client.connect(addr, 1111, 11112);
        let buf: Vec<u8> = vec![1, 2, 3, 4];
        tcp_client.send(buf.as_slice());
    }
}