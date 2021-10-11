use crate::encoding::{HandlePackage, NetworkPackage};
use crate::physical::PhysicalPackage;
use crate::redundancy::RedundancyPackage;
use crate::udp::{UDPLayer, UDPPackage};

pub struct TCPPackage {
    data: Vec<u8>,
}

impl NetworkPackage for TCPPackage {}

pub struct TCPLayer {
    udp: UDPLayer,
}

impl HandlePackage<TCPPackage> for TCPLayer {
    fn send(&mut self, package: TCPPackage) {
        todo!()
    }

    fn receive(&mut self) -> TCPPackage {
        todo!()
    }
}

impl HandlePackage<UDPPackage> for TCPLayer {
    fn send(&mut self, package: UDPPackage) {
        self.udp.send(package)
    }

    fn receive(&mut self) -> UDPPackage {
        self.udp.receive()
    }
}

impl HandlePackage<RedundancyPackage> for TCPLayer {
    fn send(&mut self, package: RedundancyPackage) {
        self.udp.send(package)
    }

    fn receive(&mut self) -> RedundancyPackage {
        self.udp.receive()
    }
}

impl HandlePackage<PhysicalPackage> for TCPLayer {
    fn send(&mut self, package: PhysicalPackage) {
        self.udp.send(package)
    }

    fn receive(&mut self) -> PhysicalPackage {
        self.udp.receive()
    }
}
