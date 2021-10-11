use crate::encoding::{HandlePackage, NetworkPackage};
use crate::physical::PhysicalPackage;
use crate::redundancy::{RedundancyLayer, RedundancyPackage};

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

impl HandlePackage<RedundancyPackage> for UDPLayer {
    fn send(&mut self, package: RedundancyPackage) {
        self.redundancy.send(package)
    }

    fn receive(&mut self) -> RedundancyPackage {
        self.redundancy.receive()
    }
}

impl HandlePackage<PhysicalPackage> for UDPLayer {
    fn send(&mut self, package: PhysicalPackage) {
        self.redundancy.send(package)
    }

    fn receive(&mut self) -> PhysicalPackage {
        self.redundancy.receive()
    }
}
