use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use cs140_util::file_io;
use cs140_project1::make_redundancy;

const SIZE: usize = 6250;
// const PATH: &str = "C:\\Users\\Leomund\\Sources\\ShanghaiTech\\cs140\\cs140-project2\\INPUT.bin";
const PATH: &str = "C:\\Users\\ViXbob\\CLionProjects\\cs140\\cs140-project2\\INPUT.bin";
#[tokio::main]
async fn main() {
    let mut builder = env_logger::Builder::from_default_env();
    builder.format_timestamp_millis().init();
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
    // let physical_layer = PhysicalLayer::new_send_only(FREQUENCY, padding + 7);
    let physical_layer = PhysicalLayer::new_with_specific_device(padding + 7, 0, 0);
    physical_layer.push_warm_up_data(100);
    let redundancy_layer = RedundancyLayer::new(physical_layer);
    let mut ip_layer = IPLayer::new(redundancy_layer);
    for p in packages {
        // println!("{:?}", p);
        ip_layer.send(IPPackage::new(p)).await;
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