use std::fs::File;
use std::io::{BufReader, Read};
use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use cs140_network::tcp::TCPLayer;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let layer = PhysicalLayer::new(2,256);
    let layer = RedundancyLayer::new(layer);
    let layer = IPLayer::new(layer,1,2);
    let layer = TCPLayer::new(layer);
    // let file = File::open(r"C:\Users\vixbo\Desktop\PackageInfo.txt").unwrap();
    // let mut reader = BufReader::new(file);
    // let mut string_to_send = String::new();
    // reader.read_to_string(&mut string_to_send);

    layer.send_raw((0..=255).cycle().take(16384).collect()).await;
    let _:String = layer.receive().await.unwrap();
}
