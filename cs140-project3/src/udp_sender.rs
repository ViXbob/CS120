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
    let dst = SocketAddr::from(SocketAddrV4::from_str("10.19.73.32:18888").unwrap());
    let socket = UdpSocket::bind("10.19.75.77:22791").await.unwrap();
    socket.send_to(data.as_slice(), dst).await;
    trace!("send completed!");
    std::thread::park();
}
