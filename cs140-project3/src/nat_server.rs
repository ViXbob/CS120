use tokio::net::UdpSocket;
use cs140_network::ip::IPLayer;
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use cs140_util::nat;
use cs140_util::nat::run_nat;
use cs140_util::rpc::{CS120ProtocolType, CS120Socket};

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let layer = PhysicalLayer::new(1, 1024);
    let layer = RedundancyLayer::new(layer);
    let mut layer = IPLayer::new(layer);
    let socket = UdpSocket::bind("10.19.73.32:18888").await.unwrap();
    run_nat(layer, socket, CS120ProtocolType::Udp).await;
    std::thread::park();
}