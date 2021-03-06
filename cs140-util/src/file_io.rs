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

    let data: Vec<_> = buffer[0..byte_size].iter().cloned().collect();

    data
}

pub fn write_bytes_into_bin_file(path: &str, data: &[u8]) {
    let mut buffer = File::create(path).unwrap();
    buffer.write(data);
}

pub fn read_bytes_from_file(path: &str) -> Vec<u8> {
    use std::fs::File;
    use std::io;
    use std::io::prelude::*;
    // let pwd = std::env::current_dir().unwrap();
    // println!("{}", pwd.to_str().unwrap());
    let mut f = File::open(path).unwrap();
    let mut buffer = [0; 100000];
    f.seek(SeekFrom::Start(0));
    let n = f.read(&mut buffer).unwrap();
    buffer[0..n].to_vec()
}
