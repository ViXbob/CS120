use async_trait::async_trait;
use bitvec::order::Msb0;
use bitvec::vec::BitVec;

use cs140_common::padding::{padding_inclusive_range, padding_range};

pub type BitStore = BitVec<Msb0, u8>;

pub trait NetworkPackage {}

#[async_trait]
pub trait HandlePackage<Package: NetworkPackage> {
    async fn send(&mut self, package: Package);
    async fn receive(&mut self) -> Package;
}

const TABLE: &'static [u8] = &[0b11110u8, 0b01001u8, 0b10100u8, 0b10101u8,
    0b01010u8, 0b01011u8, 0b01110u8, 0b01111u8,
    0b10010u8, 0b10011u8, 0b10110u8, 0b10111u8,
    0b11010u8, 0b11011u8, 0b11100u8, 0b11101u8];

pub fn encode_4b5b(data: &BitStore) -> BitStore {
    let mut result: BitStore = BitVec::with_capacity((data.len() as f64 * 1.25).floor() as usize);
    for bits in data.chunks(4) {
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
    let mut result: BitStore = BitVec::with_capacity((data.len() as f64 * 0.8).floor() as usize);
    for bits in data.chunks(5) {
        if bits.len() < 5{
            log::warn!("Fail to decode, bits len {}, which is too short.", bits.len());
            break;
        }
        let mut value: u8 = 0;
        for bit in bits {
            value <<= 1;
            value += *bit as u8;
        }
        let decoded = TABLE.iter().position(|&x| { x == value });
        if let Some(decoded) = decoded {
            for shift in (0..4).rev() {
                result.push(((decoded >> shift) & 1) == 1);
            }
        } else {
            log::warn!("Fail to decode, pushing random bits.");
            result.extend(padding_inclusive_range(0..=1).take(4).map(|x| x == 1));
        }
    }
    result
}

pub fn decode_nrzi(data: &BitStore) -> BitStore {
    let mut result: BitStore = BitVec::with_capacity(data.len());
    let mut old_bit: bool = false;
    for bit in data {
        if old_bit != *bit {
            old_bit = *bit;
            result.push(true);
        } else {
            result.push(false);
        }
    }
    result
}

pub fn encode_nrzi(data: &BitStore) -> BitStore {
    let mut result: BitStore = BitVec::with_capacity(data.len());
    let mut state: bool = false;
    for bit in data {
        if *bit {
            state = !state
        }
        result.push(state);
    }
    result
}

#[cfg(test)]
mod test {
    use crate::encoding::BitStore;

    use super::*;

    #[test]
    fn test_bitstore() {
        let original: Vec<u8> = vec![33, 44, 127, 204];
        let bv: BitStore = BitStore::from_vec(original);
        let encoded_nrzi = encode_nrzi(&bv);
        let decoded_nrzi = decode_nrzi(&encoded_nrzi);
        assert_eq!(bv, decoded_nrzi);
        let encoded_4b5b = encode_4b5b(&bv);
        let decoded_4b5b = decode_4b5b(&encoded_4b5b);
        assert_eq!(bv, decoded_4b5b);
        let encoded_4b5b_nrzi = encode_4b5b(&encoded_nrzi);
        let decoded_4b5b_nrzi = decode_4b5b(&encoded_4b5b_nrzi);
        assert_eq!(bv, decode_nrzi(&decoded_4b5b_nrzi));
    }
}
