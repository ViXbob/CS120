use crate::encoding::{HandlePackage, NetworkPackage};
use crate::ip::{IPLayer, IPPackage};
use crate::physical::PhysicalPackage;
use crate::redundancy::RedundancyPackage;
use async_trait::async_trait;

pub struct TCPPackage {
    data: Vec<u8>,
}

impl NetworkPackage for TCPPackage {}

pub struct TCPLayer {
    ip: IPLayer,
}

#[async_trait]
impl HandlePackage<TCPPackage> for TCPLayer {
    async fn send(&mut self, package: TCPPackage) {
        todo!()
    }

    async fn receive(&mut self) -> TCPPackage {
        todo!()
    }
}

#[async_trait]
impl HandlePackage<IPPackage> for TCPLayer {
    async fn send(&mut self, package: IPPackage) {
        self.ip.send(package).await
    }

    async fn receive(&mut self) -> IPPackage {
        self.ip.receive().await
    }
}

#[async_trait]
impl HandlePackage<RedundancyPackage> for TCPLayer {
    async fn send(&mut self, package: RedundancyPackage) {
        self.ip.send(package).await
    }

    async fn receive(&mut self) -> RedundancyPackage {
        self.ip.receive().await
    }

}

#[async_trait]
impl HandlePackage<PhysicalPackage> for TCPLayer {
    async fn send(&mut self, package: PhysicalPackage) {
        self.ip.send(package).await
    }

    async fn receive(&mut self) -> PhysicalPackage {
        self.ip.receive().await
    }

}
