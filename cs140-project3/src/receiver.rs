use log::trace;
use cs140_common::buffer::Buffer;
use cs140_common::descriptor::SampleFormat::F32;
use cs140_common::padding::padding_inclusive_range;
use cs140_common::record::Recorder;
use cs140_network::encoding::{BitStore, HandlePackage};
use cs140_network::ip::IPLayer;
use cs140_network::physical::PhysicalLayer;
use cs140_network::physical::PhysicalPackage;
use cs140_network::redundancy::RedundancyLayer;
use cs140_util::rpc::{CS120RPC, Transport};

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let layer = PhysicalLayer::new(1, 1024);
    let layer = RedundancyLayer::new(layer);
    let mut layer = IPLayer::new(layer);
    let package = layer.recv().await;
    trace!("{:?}", package);
    let data = match package {
        CS120RPC::UdpPackage(package) => {
            package.data
        }
        _ => {
            Vec::new()
        }
    };
    let string : &str = std::str::from_utf8(data.as_slice()).expect("couldn't convert");
    println!("{}", string);
}