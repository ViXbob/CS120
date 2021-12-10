use hound::WavWriter;

use cs140_common::buffer::Buffer;
use cs140_common::descriptor::SampleFormat::F32;
use cs140_common::padding::padding_inclusive_range;
use cs140_common::record::Recorder;
use cs140_network::encoding::{BitStore, HandlePackage};
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::physical::PhysicalPackage;
use cs140_network::redundancy::RedundancyLayer;
use cs140_network::tcp::TCPLayer;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let layer = PhysicalLayer::new(1, 256);
    let layer = RedundancyLayer::new(layer);
    let layer = IPLayer::new(layer);
    let mut layer = TCPLayer::new(layer);
    loop {
        let data:Vec<u8> = (0..=255).take(210).collect();
        layer.send(&data).await;
    }
}