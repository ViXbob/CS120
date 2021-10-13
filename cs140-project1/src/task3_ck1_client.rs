use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::{PhysicalLayer, PhysicalPackage};
use cs140_network::redundancy::{RedundancyLayer, RedundancyPackage};
use cs140_project1::erase_redundancy;
use reed_solomon_erasure::galois_8::{ReedSolomon};

fn generate_random_data() -> Vec<u8> {
    use rand::prelude::*;
    use rand_pcg::Pcg64;

    let mut rng = Pcg64::seed_from_u64(2);
    return (0..1250).map(|_| rng.gen()).collect();
}

fn main() {
    let ground_truth = generate_random_data();
    let physical_layer = PhysicalLayer::new_receive_only(&[4000.0, 5000.0], 500);
    let redundancy_layer = RedundancyLayer::new(physical_layer);
    let mut ip_layer = IPLayer::new(redundancy_layer);
    let mut data: Vec<Option<Vec<u8>>> = Vec::new();
    let data_shard_count = 6;
    let parity_shard_count = 6;
    loop {
        let package: IPPackage = ip_layer.receive();
        if package.data[0] >= data_shard_count + parity_shard_count{
            drop(package)
        }else{
            while data.len() < package.data[0] as usize{
                data.push(None);
            }
            data.push(Some(package.data));
        }
        if data.len() as u8 > data_shard_count + parity_shard_count / 2 {
            while (data.len() as u8) < (data_shard_count + parity_shard_count) {
                data.push(None);
            }
            break;
        }
    }
    let data = erase_redundancy(data, ReedSolomon::new(data_shard_count as usize, parity_shard_count as usize).unwrap(), 10000).unwrap();
    assert_eq!(data.data, generate_random_data())
    // loop{
    //     let data:RedundancyPackage = ip_layer.receive();
    //     println!("{:?}",data.data);
    // }
}
