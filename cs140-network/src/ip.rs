use bitvec::view::BitView;
use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::physical::PhysicalPackage;
use crate::redundancy::{RedundancyLayer, RedundancyPackage};
use bitvec::order::Lsb0;
use bitvec::vec::BitVec;

// data_length in [0, 65535]
pub struct IPPackage {
    data: Vec<u8>,
}

impl NetworkPackage for IPPackage {}

pub struct IPLayer {
    redundancy: RedundancyLayer,
    frame_length: usize,
}

impl IPLayer {
    fn new(redundancy: RedundancyLayer) -> Self {
        let frame_length = redundancy.physical.frame_length;
        IPLayer {
            redundancy,
            frame_length,
        }
    }
}

impl HandlePackage<IPPackage> for IPLayer {
    fn send(&mut self, package: IPPackage) {
        let frame_bytes: usize = self.frame_length / 8;
        let data = package.data.clone();
        let size = data.chunks(frame_bytes - 1).len();
        for (index, bits) in data.chunks(frame_bytes - 1).enumerate() {
            let mut tmp  = bits.to_vec();
            while tmp.len() + 1 < frame_bytes {
                tmp.push(0);
            }
            if index + 1 != size {
                tmp.push(0);
            } else {
                tmp.push(1);
            }
            self.redundancy.send(RedundancyPackage {
                data: tmp,
            });
        }
    }

    fn receive(&mut self) -> IPPackage {
        let mut data : Vec<u8> = Vec::new();
        loop {
            let package: RedundancyPackage = self.redundancy.receive();
            data.extend(package.data.iter().take(self.frame_length - 1));
            if *package.data.get(self.frame_length - 1).unwrap() == 1u8 {
                break;
            }
        }
        IPPackage {
            data,
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
