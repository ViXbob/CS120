use crate::encoding::{HandlePackage, NetworkPackage};
use crate::udp::UDPLayer;

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
