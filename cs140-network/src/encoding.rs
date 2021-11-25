use bitvec::order::Lsb0;
use bitvec::vec::BitVec;

pub type BitStore = BitVec<Lsb0, u8>;

pub trait NetworkPackage {}

use async_trait::async_trait;
use log::error;

#[async_trait]
pub trait HandlePackage<Package: NetworkPackage> {
    async fn send(&mut self, package: Package);
    async fn receive(&mut self) -> Package;
}

pub fn encode_4b5b(data: &BitStore) -> BitStore {
    const TABLE: &'static [u8] = &[0b11110u8, 0b01001u8, 0b10100u8, 0b10101u8,
        0b01010u8, 0b01011u8, 0b01110u8, 0b01111u8,
        0b10010u8, 0b10011u8, 0b10110u8, 0b10111u8,
        0b11010u8, 0b11011u8, 0b11100u8, 0b11101u8];
    let mut result: BitStore = BitVec::new();
    for bits in data.chunks(4) {
        // println!("{}", bits);
        let mut index: usize = 0;
        for bit in bits {
            index <<= 1;
            index += *bit as usize;
        }
        let encoding = TABLE[index];
        for shift in (0..5).rev() {
            result.push(((encoding >> shift) & 1) == 1);
        }
    }
    result
}

pub fn decode_4b5b(data: &BitStore) -> BitStore {
    const TABLE: &'static [u8] = &[0b11110u8, 0b01001u8, 0b10100u8, 0b10101u8,
        0b01010u8, 0b01011u8, 0b01110u8, 0b01111u8,
        0b10010u8, 0b10011u8, 0b10110u8, 0b10111u8,
        0b11010u8, 0b11011u8, 0b11100u8, 0b11101u8];
    let mut result: BitStore = BitVec::new();
    for bits in data.chunks(5) {
        // println!("{}", bits);
        let mut index: usize = 0;
        for bit in bits {
            index <<= 1;
            index += *bit as usize;
        }
        let mut decoding: usize = 16;
        for (index_1, value) in TABLE.iter().enumerate() {
            if *value as usize == index {
                decoding = index_1;
                break;
            }
        }
        if decoding > 15 {
            // error!("decode error on {}!", index);
        }
        for shift in (0..4).rev() {
            result.push(((decoding >> shift) & 1) == 1);
        }
    }
    result
}

#[cfg(test)]
mod test {
    use std::env;
    use bitvec::bitvec;
    use bitvec::prelude::Lsb0;
    use crate::encoding::{BitStore, encode_4b5b};
    use bitvec::vec::BitVec;

    #[test]
    fn test_4b5b_encoding() {
        let mut data: BitStore = BitVec::new();
        for i in (0..8) {
            data.push((i & 1) == 1);
        }
        // println!("{:?}", encode_4b5b(&data));
        let result = encode_4b5b(&data);

        assert!(result == bitvec![0, 1, 0, 1, 1, 0, 1, 0, 1, 1]);
    }
}
