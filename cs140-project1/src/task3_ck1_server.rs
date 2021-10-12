use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::{RedundancyLayer, RedundancyPackage};
use rand::Rng;

fn generate_random_data() -> Vec<u8> {
    use rand::prelude::*;
    use rand_pcg::Pcg64;

    let mut rng = Pcg64::seed_from_u64(2);
    return (0..2500).map(|_| rng.gen()).collect();
}

fn main() {
    let data = generate_random_data();
    for (index, data) in data.iter().enumerate() {
        println!("{},{}", index, data)
    }
    let data = generate_random_data();
    let physical_layer = PhysicalLayer::new_send_only(&[4000.0, 5000.0], 500);
    let redundancy_layer = RedundancyLayer::new(physical_layer);
    let mut ip_layer = IPLayer::new(redundancy_layer);
    ip_layer.send(IPPackage::new(data));
    std::thread::park();
}
