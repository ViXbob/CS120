use std::net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4};
use std::str::FromStr;
use log::trace;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use cs140_util::icmp::{AudioPinger, AudioPingUtil};

const PING_COUNT: usize = 5;

fn read(buf: &mut String) {
    buf.clear();
    std::io::stdin().read_line(buf);
    let tmp = buf.trim().clone();
    *buf = String::from(tmp);
}

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();

    let mut pinger = AudioPingUtil::new();
    let mut buf: String = String::new();

    loop {
        println!("please type the ping address,");
        read(&mut buf);
        for _ in 0..PING_COUNT {
            let addr = buf.parse::<Ipv4Addr>().unwrap();
            pinger.ping_once(addr).await;
        }
    }

    std::thread::park();
}
