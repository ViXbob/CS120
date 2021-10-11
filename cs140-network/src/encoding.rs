use bitvec::order::Lsb0;
use bitvec::vec::BitVec;

pub type BitStore = BitVec<Lsb0, u8>;

pub trait NetworkPackage {}

pub trait HandlePackage<Package: NetworkPackage> {
    fn send(&mut self, package: Package);
    fn receive(&mut self) -> Package;
}

#[cfg(test)]
mod test {
    use bitvec::vec::BitVec;
    use crate::encoding::BitStore;

    #[test]
    fn test_bitstore() {
        let mut bv : BitStore = BitVec::new();
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