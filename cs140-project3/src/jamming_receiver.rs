use cs140_network::ip::IPLayer;
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use cs140_network::tcp::TCPLayer;

#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
    let layer = PhysicalLayer::new(2,32);
    let layer = RedundancyLayer::new(layer);
    let layer = IPLayer::new(layer,2,1);
    let layer = TCPLayer::new(layer);

    let result:Option<String> = layer.receive().await;
    println!("{}",result.unwrap())
}