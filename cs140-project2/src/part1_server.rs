use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use cs140_util::file_io;
use cs140_project1::make_redundancy;

const SIZE: usize = 6250;
const PATH: &str = "/Users/vixbob/cs140/cs140-project2/INPUT.bin";

fn main() {
    const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0, 13000.0, 14000.0, 15000.0, 16000.0];
    // const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0, 9000.0, 10000.0, 11000.0, 12000.0];
    // const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0];
    // const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0];
    // const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0];
    // const FREQUENCY: &'static [f32] = &[4000.0, 5000.0];
    // const FREQUENCY: &'static [f32] = &[4000.0];
    let data = file_io::read_bytes_from_bin_file(PATH, SIZE);
    println!("{:?}", data);
    // let tmp = data;
    let padding = 65;
    // let data : Vec<_> = (0..5000).map(|x| *tmp.get(x % padding).unwrap()).collect();
    let (packages, r) = make_redundancy(data, padding, 0.4);

    println!(
        "data_shard_count: {}, parity_shard_count: {}",
        r.data_shard_count(),
        r.parity_shard_count()
    );
    let physical_layer = PhysicalLayer::new_send_only(FREQUENCY, padding + 7);
    // let physical_layer = PhysicalLayer::new_with_specific_device(FREQUENCY, padding + 7, 0);
    physical_layer.push_warm_up_data();
    let redundancy_layer = RedundancyLayer::new(physical_layer);
    let mut ip_layer = IPLayer::new(redundancy_layer);
    for p in packages {
        // println!("{:?}", p);
        ip_layer.send(IPPackage::new(p));
    }
    // ip_layer.send(IPPackage::new(data));
    println!("OK");
    std::thread::park();
}

#[cfg(test)]
mod test {
    use cs140_util::file_io;

    #[test]
    fn test_read_bin() {
        let data = file_io::read_bytes_from_bin_file("INPUT.bin", 6250);
        println!("{:?}", data);
    }
}