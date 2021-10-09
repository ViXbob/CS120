use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::physical::{PhysicalLayer, PhysicalPackage};
use bitvec::prelude::BitVec;

pub struct RedundancyPackage {
    data: Vec<u8>,
}

impl NetworkPackage for RedundancyPackage {}

pub struct RedundancyLayer {
    physical: PhysicalLayer,
}

impl RedundancyLayer {
    fn make_redundancy(&self, package: RedundancyPackage) -> BitStore {
        return BitVec::from_vec(package.data);
    }

    fn erase_redundancy(&self, data: BitStore) -> Option<RedundancyPackage> {
        return Some(RedundancyPackage {
            data: data.into_vec(),
        });
    }
}

impl HandlePackage<RedundancyPackage> for RedundancyLayer {
    fn send(&mut self, package: RedundancyPackage) {
        self.physical.send(PhysicalPackage {
            0: self.make_redundancy(package),
        })
    }

    fn receive(&mut self) -> RedundancyPackage {
        loop {
            let result = self.physical.receive().0;
            let result = self.erase_redundancy(result);
            if let Some(result) = result {
                return result;
            }
        }
    }
}
