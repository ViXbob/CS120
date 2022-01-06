use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use log::trace;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use cs140_util::icmp::AudioPinger;

const PING_COUNT: usize = 5;


#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let layer = PhysicalLayer::new(1, 128);
    let layer = RedundancyLayer::new(layer);
    let mut layer = IPLayer::new(layer);

    let mut pinger = AudioPinger::new(layer, 0x0002);

    for _ in 0..PING_COUNT {
        // let addr = std::net::Ipv4Addr::new(10, 11, 128, 69);
        // 220.181.38.148
        // pinger.ping_once(Ipv4Addr::from_str("10.20.210.29").unwrap()).await;
        pinger.ping_once(Ipv4Addr::from_str("64.99.80.121").unwrap()).await;
        // pinger.ping_once(IpAddr::from_str("10.11.128.69").unwrap()).await;
    }

    std::thread::park();
}
