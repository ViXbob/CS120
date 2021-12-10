use std::net::{SocketAddr, SocketAddrV4};
use std::str::FromStr;
use log::trace;
use tokio::net::UdpSocket;
use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use cs140_util::file_io;
use cs140_util::rpc::{CS120RPC, Transport, UdpPackage};


const SIZE: usize = 6250;
const PATH: &str = "INPUT.bin";

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let data = file_io::read_bytes_from_bin_file(PATH, SIZE);
    trace!("{:?}", data);
    let layer = PhysicalLayer::new(16, 64);
    let layer = RedundancyLayer::new(layer);
    let mut layer = IPLayer::new(layer);
    let src = SocketAddr::from(SocketAddrV4::from_str("192.168.1.2:1234").unwrap());
    let dst = SocketAddr::from(SocketAddrV4::from_str("10.19.75.77:28888").unwrap());
    let package = CS120RPC::UdpPackage(UdpPackage{src, dst, data});
    layer.trans(package).await;
    trace!("send completed!");
    std::thread::park();
}
