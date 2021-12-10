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
    let layer = PhysicalLayer::new(1, 32);
    let layer = RedundancyLayer::new(layer);
    let mut layer = IPLayer::new(layer);

    let mut pinger = AudioPinger::new(layer, 0x0002);

    for _ in 0..PING_COUNT {
        pinger.ping_once(IpAddr::from_str("220.181.38.148").unwrap()).await;
    }

    std::thread::park();
}
