use std::net::{IpAddr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use log::trace;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use cs140_util::icmp::AudioPinger;

const PING_COUNT: usize = 20;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let layer = PhysicalLayer::new(1, 128);
    let layer = RedundancyLayer::new(layer);
    let mut layer = IPLayer::new(layer);

    let mut ping_replyer = AudioPinger::new(layer, 0x0002);

    ping_replyer.wait_icmp_request_and_reply().await;

    std::thread::park();
}
