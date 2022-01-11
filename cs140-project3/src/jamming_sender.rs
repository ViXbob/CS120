use std::fs::File;
use std::io::{BufReader, Read};
use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use cs140_network::tcp::TCPLayer;
use cs140_util::file_io;

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
    let data = file_io::read_bytes_from_bin_file("INPUT.bin", 6250);

    layer.send_raw(data).await;
}
