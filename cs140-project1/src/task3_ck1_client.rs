use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::{PhysicalLayer, PhysicalPackage};
use cs140_network::redundancy::{RedundancyLayer, RedundancyPackage};
use cs140_project1::erase_redundancy;
use reed_solomon_erasure::galois_8::ReedSolomon;

fn generate_random_data() -> Vec<u8> {
    use rand::prelude::*;
    use rand_pcg::Pcg64;

    let mut rng = Pcg64::seed_from_u64(2);
    return (0..1250).map(|_| rng.gen()).collect();
}

fn main() {
    const BYTE_IN_FRAME: usize = 53;
    let ground_truth = generate_random_data();
    let physical_layer = PhysicalLayer::new_receive_only(&[4000.0, 5000.0], BYTE_IN_FRAME);
    let redundancy_layer = RedundancyLayer::new(physical_layer);
    let mut ip_layer = IPLayer::new(redundancy_layer);
    let mut data: Vec<Option<Vec<u8>>> = Vec::new();
    let data_shard_count = 27;
    let parity_shard_count = 27;
    let mut package_received = 0;
    loop {
        let package: IPPackage = ip_layer.receive();
        // println!("received: {}", package.data[0]);
        if package.data[0] >= data_shard_count + parity_shard_count {
            // println!("data corrupted, maximum index {}, found {}", data_shard_count  + parity_shard_count, package.data[0]);
            drop(package)
        } else if package.data.len() != BYTE_IN_FRAME - 3 {
            // println!("invalid length {} for package",package.data.len());
            drop(package)
        } else {
            // check whether the package is corrupted
            if package.data[1..package.data.len()]
                .iter()
                .fold(0, |old, x| old ^ x)
                != 0
            {
                // println!("data corrupted")
            } else {
                package_received += 1;
                // println!("now we received {} packages",package_received);
                while data.len() < package.data[0] as usize {
                    data.push(None);
                }
                data.push(Some(package.data));
            }
        }
        if package_received >= data_shard_count {
            while (data.len() as u8) < (data_shard_count + parity_shard_count) {
                data.push(None);
            }
            break;
        }
    }
    let data = erase_redundancy(
        data,
        ReedSolomon::new(data_shard_count as usize, parity_shard_count as usize).unwrap(),
        1250,
    )
    .unwrap();
    let error_count = data
        .iter()
        .zip(ground_truth.iter())
        .fold(0, |old, (received, excepted)| {
            if received != excepted {
                println!("error: received {}, excepted {}", received, excepted);
                old + 1
            } else {
                old
            }
        });
    println!("total error count: {}", error_count);
    assert_eq!(data, ground_truth)
    // loop{
    //     let data:RedundancyPackage = ip_layer.receive();
    //     println!("{:?}",data.data);
    // }
}
