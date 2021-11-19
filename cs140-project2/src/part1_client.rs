use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::{PhysicalLayer, PhysicalPackage};
use cs140_network::redundancy::{RedundancyLayer, RedundancyPackage};
use cs140_project1::{erase_redundancy, read_bits_from_file};
use reed_solomon_erasure::galois_8::ReedSolomon;
use cs140_util::file_io;

fn generate_random_data() -> Vec<u8> {
    use rand::prelude::*;
    use rand_pcg::Pcg64;

    let mut rng = Pcg64::seed_from_u64(2);
    return (0..1250).map(|_| rng.gen()).collect();
}

const SIZE: usize = 6250;
// const PATH: &str = "/Users/vixbob/cs140/cs140-project2/OUTPUT.bin";
const PATH: &str = "C:\\Users\\Leomund\\Sources\\ShanghaiTech\\cs140\\cs140-project2\\OUTPUT.bin";
fn main() {
    const BYTE_IN_FRAME: usize = 7 + 65;
    const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0, 13000.0, 14000.0, 15000.0, 16000.0];
    // const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0];
    // const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0];
    // const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0];
    // const FREQUENCY: &'static [f32] = &[4000.0, 5000.0];
    // const FREQUENCY: &'static [f32] = &[4000.0];
    // let physical_layer = PhysicalLayer::new_receive_only(FREQUENCY, BYTE_IN_FRAME);
    let physical_layer = PhysicalLayer::new_with_specific_device(FREQUENCY, BYTE_IN_FRAME, 0);
    // let physical_layer = PhysicalLayer::new(FREQUENCY, BYTE_IN_FRAME);
    // let physical_layer = PhysicalLayer::new_with_specific_device(FREQUENCY, BYTE_IN_FRAME, 0);
    let redundancy_layer = RedundancyLayer::new(physical_layer);
    let mut ip_layer = IPLayer::new(redundancy_layer);
    let mut data: Vec<Option<Vec<u8>>> = Vec::new();
    let data_shard_count = 100;
    let parity_shard_count = 40;
    let mut package_received = 0;
    let mut now_package = 0;
    loop {
        let package: IPPackage = ip_layer.receive();
        // println!("received: {}", package.data[0]);
        now_package += 1;
        if package.data[0] >= data_shard_count + parity_shard_count {
            // println!("data corrupted, maximum index {}, found {}", data_shard_count  + parity_shard_count, package.data[0]);
            drop(package)
        } else if package.data.len() != BYTE_IN_FRAME - 7 {
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
                // println!("now total {} packages",now_package);
                println!("now we received {} packages",package_received);
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
        SIZE,
    ).unwrap();
    file_io::write_bytes_into_bin_file(PATH, data.as_slice());
}
