use crate::encoding::{HandlePackage, NetworkPackage};
use crate::redundancy::RedundancyLayer;

pub struct UDPPackage {
    data: Vec<u8>,
}

impl NetworkPackage for UDPPackage {}

pub struct UDPLayer {
    redundancy: RedundancyLayer,
}

impl HandlePackage<UDPPackage> for UDPLayer {
    fn send(&mut self, package: UDPPackage) {
        todo!()
    }

    fn receive(&mut self) -> UDPPackage {
        todo!()
    }
}
