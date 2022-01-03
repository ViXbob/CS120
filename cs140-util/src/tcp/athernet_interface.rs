use smoltcp::phy::{self, Device, DeviceCapabilities, Medium};
use smoltcp::time::Instant;
use std::sync::Arc;
use std::task::Poll;
use tokio::runtime::Handle;
use tokio::time::error::Elapsed;
use cs140_network::ip::{IPLayer, IPPackage};
use cs140_network::physical::PhysicalLayer;
use cs140_network::redundancy::RedundancyLayer;
use crate::rpc::Transport;

pub struct AthernetInterface {
    layer: Arc<IPLayer>,
    mtu: usize,
    medium: Medium,
}

impl AthernetInterface {
    pub fn new(mtu: usize, medium: Medium) -> Self {
        let layer = PhysicalLayer::new(16, mtu);
        let layer = RedundancyLayer::new(layer);
        let layer = IPLayer::new(layer);
        let layer = Arc::new(layer);
        AthernetInterface {
            layer,
            mtu,
            medium,
        }
    }
}

impl<'a> Device<'a> for AthernetInterface {
    type RxToken = RxToken;
    type TxToken = TxToken;

    fn receive(&'a mut self) -> Option<(Self::RxToken, Self::TxToken)> {
        let mut layer = self.layer.clone();
        let handle = tokio::runtime::Handle::current();
        handle.enter();
        let result = futures::executor::block_on(async move{
            tokio::time::timeout(std::time::Duration::from_micros(100),layer.recv_package()).await
        });
        match result {
            Ok(buffer) => {
                Some((RxToken {buffer}, TxToken {layer: self.layer.clone()}))
            }
            Err(_) => {
                None
            }
        }
    }

    fn transmit(&'a mut self) -> Option<Self::TxToken> {
        Some(TxToken {
            layer: self.layer.clone()
        })
    }

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.max_transmission_unit = self.mtu;
        caps.max_burst_size = Some(1);
        caps.medium =  self.medium;
        caps
    }
}

pub struct RxToken {
    buffer: Vec<u8>,
}

impl phy::RxToken for RxToken {
    fn consume<R, F>(mut self, timestamp: smoltcp::time::Instant, f: F) -> smoltcp::Result<R> where F: FnOnce(&mut [u8]) -> smoltcp::Result<R> {
        f(&mut self.buffer[..])
    }
}

pub struct TxToken {
    layer: Arc<IPLayer>,
}

impl phy::TxToken for TxToken {
    fn consume<R, F>(self, timestamp: smoltcp::time::Instant, len: usize, f: F) -> smoltcp::Result<R> where F: FnOnce(&mut [u8]) -> smoltcp::Result<R> {
        let mut layer = self.layer.clone();
        let mut buffer = vec![0; len];
        let result = f(&mut buffer);
        // layer.send_package(buffer).await;
        let handle = Handle::current();
        handle.enter();
        futures::executor::block_on(async move{
            layer.send_package(buffer).await
        });
        result
    }
}


