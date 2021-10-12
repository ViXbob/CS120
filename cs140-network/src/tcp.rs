use crate::encoding::{HandlePackage, NetworkPackage};
use crate::physical::PhysicalPackage;
use crate::redundancy::RedundancyPackage;
use crate::ip::{IPLayer, IPPackage};

pub struct TCPPackage {
    data: Vec<u8>,
}

impl NetworkPackage for TCPPackage {}

pub struct TCPLayer {
    ip: IPLayer,
}

impl HandlePackage<TCPPackage> for TCPLayer {
    fn send(&mut self, package: TCPPackage) {
        todo!()
    }

    fn receive(&mut self) -> TCPPackage {
        todo!()
    }
}

impl HandlePackage<IPPackage> for TCPLayer {
    fn send(&mut self, package: IPPackage) {
        self.ip.send(package)
    }

    fn receive(&mut self) -> IPPackage {
        self.ip.receive()
    }
}

impl HandlePackage<RedundancyPackage> for TCPLayer {
    fn send(&mut self, package: RedundancyPackage) {
        self.ip.send(package)
    }

    fn receive(&mut self) -> RedundancyPackage {
        self.ip.receive()
    }
}

impl HandlePackage<PhysicalPackage> for TCPLayer {
    fn send(&mut self, package: PhysicalPackage) {
        self.ip.send(package)
    }

    fn receive(&mut self) -> PhysicalPackage {
        self.ip.receive()
    }
}
