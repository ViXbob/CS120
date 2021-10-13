use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::{RedundancyLayer, RedundancyPackage};
use rand::Rng;
use cs140_project1::make_redundancy;

fn generate_random_data() -> Vec<u8> {
    use rand::prelude::*;
    use rand_pcg::Pcg64;

    let mut rng = Pcg64::seed_from_u64(2);
    return (0..1250).map(|_| rng.gen()).collect();
}

fn main() {
    let data = generate_random_data();
    let padding = 497;
    let (packages,r) = make_redundancy(data,padding,1.0);
    println!("data_shard_count: {}, parity_shard_count: {}",r.data_shard_count(),r.parity_shard_count());
    let physical_layer = PhysicalLayer::new_send_only(&[4000.0, 5000.0], 500);
    let redundancy_layer = RedundancyLayer::new(physical_layer);
    let mut ip_layer = IPLayer::new(redundancy_layer);
    for p in packages{
        ip_layer.send(IPPackage::new(p));
    }
    std::thread::park();
}
