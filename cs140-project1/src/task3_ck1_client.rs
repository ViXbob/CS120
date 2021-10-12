use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::{PhysicalLayer, PhysicalPackage};
use cs140_network::redundancy::{RedundancyLayer, RedundancyPackage};

fn generate_random_data() -> Vec<u8> {
    use rand::prelude::*;
    use rand_pcg::Pcg64;

    let mut rng = Pcg64::seed_from_u64(2);
    return (0..2500).map(|_|rng.gen()).collect();
}

fn main() {
    let ground_truth = generate_random_data();
    let physical_layer = PhysicalLayer::new_receive_only(&[4000.0, 5000.0], 500);
    let redundancy_layer = RedundancyLayer::new(physical_layer);
    let mut ip_layer = IPLayer::new(redundancy_layer);
    let mut index = 0;
    let data:IPPackage = ip_layer.receive();
    assert_eq!(data.data,generate_random_data())
    // loop{
    //     let data:RedundancyPackage = ip_layer.receive();
    //     println!("{:?}",data.data);
    // }
}
