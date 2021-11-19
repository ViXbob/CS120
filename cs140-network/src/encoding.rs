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
}
