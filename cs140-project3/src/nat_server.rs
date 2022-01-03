use tokio::net::UdpSocket;
use cs140_network::ip::IPLayer;
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use cs140_util::icmp::IcmpSocket;
use cs140_util::nat;
use cs140_util::nat::run_nat;
use cs140_util::rpc::{CS120ProtocolType, CS120Socket};

#[tokio::main(flavor = "multi_thread", worker_threads = 64)]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let layer = PhysicalLayer::new(1, 128);
    let layer = RedundancyLayer::new(layer);
    let layer = IPLayer::new(layer);
    // let socket = UdpSocket::bind("10.19.73.32:18888").await.unwrap();
    // run_nat(layer, socket, CS120ProtocolType::Udp).await;
    // let socket = IcmpSocket::new();
    let socket = UdpSocket::bind("10.19.73.32:18888").await.unwrap();
    run_nat(layer, socket, CS120ProtocolType::Icmp).await;
    // let socket = UdpSocket::bind("10.19.73.32:18888").await.unwrap();
    // run_nat(layer, socket, CS120ProtocolType::IcmpEchoRequest).await;
    // tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    // println!("GGGG");
    std::thread::park();
}