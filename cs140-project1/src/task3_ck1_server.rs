use cs140_network::encoding::HandlePackage;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::{RedundancyLayer, RedundancyPackage};
use cs140_project1::make_redundancy;
use cs140_project1::read_bits_from_file;
use rand::Rng;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::SeekFrom;

fn generate_random_data() -> Vec<u8> {
    use rand::prelude::*;
    use rand_pcg::Pcg64;

    let mut rng = Pcg64::seed_from_u64(2);
    return (0..1250).map(|_| rng.gen()).collect();
}

const SIZE: usize = 10000;
const PATH: &str = "INPUT.txt";

fn main() {
    let data = read_bits_from_file(PATH, SIZE);
    println!("{:?}", data);
    let padding = 50;
    let (packages, r) = make_redundancy(data, padding, 1.0);
    println!(
        "data_shard_count: {}, parity_shard_count: {}",
        r.data_shard_count(),
        r.parity_shard_count()
    );
    let physical_layer = PhysicalLayer::new_send_only(&[4000.0, 5000.0], 53);
    let redundancy_layer = RedundancyLayer::new(physical_layer);
    let mut ip_layer = IPLayer::new(redundancy_layer);
    for p in packages {
        ip_layer.send(IPPackage::new(p));
    }
    std::thread::park();
}

#[cfg(test)]
mod test {
    use crate::{read_bits_from_file, PATH, SIZE};
    use std::fs::File;
    use std::io;
    use std::io::prelude::*;
    use std::io::SeekFrom;

    #[test]
    fn file_io_test() -> io::Result<()> {
        let pwd = std::env::current_dir().unwrap();
        println!("{}", pwd.to_str().unwrap());
        let mut f = File::open("foo.txt")?;
        let mut buffer = [0; 10];

        // skip to the last 10 bytes of the file
        f.seek(SeekFrom::Start(0))?;

        // read up to 10 bytes
        let n = f.read(&mut buffer)?;

        println!("The bytes: {:?}", &buffer[..n]);
        Ok(())
    }

    #[test]
    fn read_from_bit_file() {
        let data = read_bits_from_file(PATH, SIZE);
        println!("{:?}", data);
    }

    #[test]
    fn write_to_file() -> std::io::Result<()> {
        let data = b"1ome bytes";

        let mut pos = 0;
        let mut buffer = File::create("foo.txt")?;

        while pos < data.len() {
            let bytes_written = buffer.write(&data[pos..])?;
            pos += bytes_written;
        }
        Ok(())
    }
}
