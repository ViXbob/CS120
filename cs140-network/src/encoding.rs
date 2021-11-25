use bitvec::order::Lsb0;
use bitvec::vec::BitVec;

pub type BitStore = BitVec<Lsb0, u8>;

pub trait NetworkPackage {}

use async_trait::async_trait;

#[async_trait]
pub trait HandlePackage<Package: NetworkPackage> {
    async fn send(&mut self, package: Package);
    async fn receive(&mut self) -> Package;
}

pub fn encode_4b5b(data: &BitStore) -> BitStore {
    const TABLE: &'static [u8] =  &[0b11110u8, 0b01001u8, 0b10100u8, 0b10101u8,
        0b01010u8, 0b01011u8, 0b01110u8, 0b01111u8,
        0b10010u8, 0b10011u8, 0b10110u8, 0b10111u8,
        0b11010u8, 0b11011u8, 0b11100u8, 0b11101u8];

}

#[cfg(test)]
mod test {
    use crate::encoding::BitStore;
    use bitvec::vec::BitVec;

    #[test]
    fn test_bitstore() {
        let mut bv: BitStore = BitVec::new();
        bv.push(false);
        bv.push(true);
        bv.push(false);
        for (index, bits) in bv.chunks(2).enumerate() {
            println!("chunk index: {}", index);
            for (i, bit) in bits.iter().enumerate() {
                println!("{}, {}", i, bit);
            }
        }
    }

    #[test]
    fn test_4b5b() {
        const TABLE: &'static [u8] =  &[0b11110u8, 0b01001u8, 0b10100u8, 0b10101u8,
                                        0b01010u8, 0b01011u8, 0b01110u8, 0b01111u8,
                                        0b10010u8, 0b10011u8, 0b10110u8, 0b10111u8,
                                        0b11010u8, 0b11011u8, 0b11100u8, 0b11101u8];
        for code in TABLE {
            println!("{}", code);
        }
    }
}
