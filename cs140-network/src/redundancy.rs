use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::physical::{PhysicalLayer, PhysicalPackage};
use async_trait::async_trait;
use bitvec::prelude::BitVec;

pub struct RedundancyPackage {
    pub data: Vec<u8>,
}

impl NetworkPackage for RedundancyPackage {}

pub struct RedundancyLayer {
    pub(crate) physical: PhysicalLayer,
    pub(crate) byte_in_frame: usize,
}

impl RedundancyLayer {
    pub fn new(physical: PhysicalLayer) -> Self {
        let byte_in_frame = physical.byte_in_frame;
        Self {
            physical,
            byte_in_frame,
        }
    }

    fn make_redundancy(&self, package: RedundancyPackage) -> BitStore {
        BitVec::from_vec(package.data)
    }

    fn erase_redundancy(&self, data: BitStore) -> Option<RedundancyPackage> {
        Some(RedundancyPackage {
            data: data.into_vec(),
        })
    }
}
#[async_trait]
impl HandlePackage<RedundancyPackage> for RedundancyLayer {
    async fn send(&mut self, package: RedundancyPackage) {
        self.physical
            .send(PhysicalPackage {
                0: self.make_redundancy(package),
            })
            .await
    }

    async fn receive(&mut self) -> RedundancyPackage {
        loop {
            let result = self.physical.receive().await.0;
            let result = self.erase_redundancy(result);
            if let Some(result) = result {
                return result;
            }
        }
    }
}
#[async_trait]
impl HandlePackage<PhysicalPackage> for RedundancyLayer {
    async fn send(&mut self, package: PhysicalPackage) {
        self.physical.send(package).await
    }

    async fn receive(&mut self) -> PhysicalPackage {
        self.physical.receive().await
    }
}
