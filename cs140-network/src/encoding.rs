use bitvec::order::Lsb0;
use bitvec::vec::BitVec;

pub trait NetworkPackage {}

pub trait HandlePackage<Package: NetworkPackage> {
    fn send(&mut self, package: Package);
    fn receive(&mut self) -> Package;
}

pub type BitStore = BitVec<Lsb0, u8>;
