use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::physical::PhysicalPackage;
use crate::redundancy::{RedundancyLayer, RedundancyPackage};
use bitvec::order::Lsb0;
use bitvec::vec::BitVec;
use bitvec::view::BitView;

pub struct IPPackage {
    pub data: Vec<u8>,
}

impl IPPackage {
    pub fn new(data: Vec<u8>) -> Self {
        assert!(data.len() < 65534);
        Self { data }
    }
}

impl NetworkPackage for IPPackage {}

pub struct IPLayer {
    redundancy: RedundancyLayer,
    byte_per_frame: usize,
}

impl IPLayer {
    pub fn new(redundancy: RedundancyLayer) -> Self {
        let frame_length = redundancy.physical.frame_length;
        IPLayer {
            redundancy,
            byte_per_frame: frame_length / 8,
        }
    }
}

impl HandlePackage<IPPackage> for IPLayer {
    fn send(&mut self, package: IPPackage) {
        let byte_per_frame: usize = self.byte_per_frame;
        let chunks = package.data.chunks(byte_per_frame - 2);
        for (index, ip_data) in chunks.enumerate() {
            let mut data = Vec::with_capacity(byte_per_frame);
            let len = ip_data.len() as u16;
            data.push((len & 0xff00 >> 8) as u8);
            data.push((len & 0x00ff) as u8);
            data.extend(ip_data.into_iter());
            data.resize(byte_per_frame, 0);
            self.redundancy.send(RedundancyPackage { data });
            println!("Package {} sent, len {}.", index, len)
        }
    }

    fn receive(&mut self) -> IPPackage {
        let mut data: Vec<u8> = Vec::new();
        loop {
            let package: RedundancyPackage = self.redundancy.receive();
            assert_eq!(package.data.len(), self.byte_per_frame);
            let len = (package.data[0] as usize) << 8 + package.data[1] as usize;
            data.extend(package.data.into_iter().skip(2).take(len));
            if len == self.byte_per_frame - 2 {
                return IPPackage { data };
            }
        }
    }
}

impl HandlePackage<RedundancyPackage> for IPLayer {
    fn send(&mut self, package: RedundancyPackage) {
        self.redundancy.send(package)
    }

    fn receive(&mut self) -> RedundancyPackage {
        self.redundancy.receive()
    }
}

impl HandlePackage<PhysicalPackage> for IPLayer {
    fn send(&mut self, package: PhysicalPackage) {
        self.redundancy.send(package)
    }

    fn receive(&mut self) -> PhysicalPackage {
        self.redundancy.receive()
    }
}
