use async_trait::async_trait;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;

use crate::encoding::{HandlePackage, HandlePackageMut, NetworkPackage};
use crate::redundancy::{BYTE_IN_ADDRESS, BYTE_IN_ENDING, BYTE_IN_LENGTH, RedundancyLayer, RedundancyPackage};

#[derive(Debug, Clone)]
pub struct IPPackage {
    pub data: Vec<u8>,
}

impl IPPackage {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl NetworkPackage for IPPackage {}

pub struct IPLayer {
    byte_in_frame: usize,
    send_package_sender: Sender<RedundancyPackage>,
    recv_package_receiver: Mutex<Receiver<RedundancyPackage>>,
}

impl IPLayer {
    pub fn new(mut redundancy: RedundancyLayer) -> Self {
        let byte_in_frame = redundancy.byte_in_frame;
        let (send_package_sender, mut send_package_receiver) = tokio::sync::mpsc::channel::<RedundancyPackage>(2);
        let (recv_package_sender, recv_package_receiver) = tokio::sync::mpsc::channel::<RedundancyPackage>(1024);
        tokio::spawn(async move{
            loop{
                tokio::select! {
                    package = send_package_receiver.recv() => {
                        match package {
                            None => {
                                return;
                            }
                            Some(package) => {
                                redundancy.send(package).await;
                            }
                        }
                    }
                    package = redundancy.receive() => {
                        if let Err(_) = recv_package_sender.send(package).await{
                            return;
                        }
                    }
                }
            }
        });
        IPLayer {
            byte_in_frame,
            send_package_sender,
            recv_package_receiver: Mutex::new(recv_package_receiver),
        }
    }
}

#[async_trait]
impl HandlePackage<IPPackage> for IPLayer {
    async fn send(&self, package: IPPackage) {
        let chunks = package.data.chunks(self.byte_in_frame);
        let last_chunk_index = chunks.len() - 1;
        for (index, ip_data) in chunks.enumerate() {
            let package = RedundancyPackage::new(ip_data.iter().cloned(),ip_data.len(),index != last_chunk_index,0,0);
            self.send_package_sender.send(package).await.unwrap();
        }
    }

    async fn receive(&self) -> IPPackage {
        let mut data: Vec<u8> = Vec::new();
        loop {
            let package: RedundancyPackage = self.recv_package_receiver.lock().await.recv().await.unwrap();
            let len = package.len();
            let more_fragments = package.has_more_fragments();
            data.extend(package.data.into_iter().skip(BYTE_IN_LENGTH + BYTE_IN_ENDING + BYTE_IN_ADDRESS).take(len));
            if !more_fragments {
                return IPPackage { data };
            }
        }
    }
}