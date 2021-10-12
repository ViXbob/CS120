use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;

fn generate_random_data() -> Vec<u8> {
    return (0..10000).map(|x: u32| (x % 2) as u8).collect();
}

fn main() {
    let data = generate_random_data();
    let physical_layer = PhysicalLayer::new(&[4000.0, 5000.0], 10024);
    let redundancy_layer = RedundancyLayer::new(physical_layer);
    let mut ip_layer = IPLayer::new(redundancy_layer);
    ip_layer.send(IPPackage::new(data));
}
