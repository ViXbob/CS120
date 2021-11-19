use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::SeekFrom;

pub fn read_bytes_from_bin_file(path: &str, byte_size: usize) -> Vec<u8> {
    // let pwd = std::env::current_dir().unwrap();
    // println!("{}", pwd.to_str().unwrap());
    let mut f = File::open(path).unwrap();
    let mut buffer = [0; 100000];
    f.seek(SeekFrom::Start(0));
    let n = f.read(&mut buffer).unwrap();

    let data:Vec<_> = buffer[0..byte_size].iter().cloned().collect();

    data
}

pub fn write_bytes_into_bin_file(path: &str, data: &[u8]) {
    let mut buffer = File::create(path).unwrap();
    buffer.write(data);
}

pub fn read_bits_from_file(path: &str, size: usize) -> Vec<u8> {
    use std::fs::File;
    use std::io;
    use std::io::prelude::*;
    use std::io::SeekFrom;
    // let pwd = std::env::current_dir().unwrap();
    // println!("{}", pwd.to_str().unwrap());
    let mut f = File::open(path).unwrap();
    let mut buffer = [0; 100000];
    f.seek(SeekFrom::Start(0));
    let n = f.read(&mut buffer).unwrap();

    let data: Vec<_> = buffer[..size]
        .chunks(8)
        .map(|bits: _| {
            let mut value: u8 = 0;
            for (index, bit) in bits.iter().enumerate() {
                value += ((bit - 48) << (7 - index));
            }
            value
        })
        .collect();
    data
}
