use async_trait::async_trait;
use bincode::{Decode, Encode};
use bincode::config::Configuration;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;

use log::trace;

use crate::encoding::{HandlePackage, HandlePackageMut, NetworkPackage};
use crate::redundancy::{BYTE_IN_ADDRESS, BYTE_IN_ENDING, BYTE_IN_LENGTH, RedundancyLayer, RedundancyPackage};
use crate::tcp::TCPPackage;

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
    pub(crate) byte_in_frame: usize,
    send_package_sender: Sender<IPPackage>,
    recv_package_receiver: Mutex<Receiver<IPPackage>>,
}

impl IPLayer {
    pub fn new(mut redundancy: RedundancyLayer, src_address: u8, dest_address: u8) -> Self {
        let byte_in_frame = redundancy.byte_in_frame;
        let (send_package_sender, mut send_package_receiver) = tokio::sync::mpsc::channel::<IPPackage>(1);
        let (recv_package_sender, recv_package_receiver) = tokio::sync::mpsc::channel::<IPPackage>(1024);

        tokio::spawn(async move {
            let mut data: Vec<u8> = Vec::new();
            loop {
                tokio::select! {
                    package = send_package_receiver.recv() => {
                        match package {
                            None => {
                                return;
                            }
                            Some(package) => {
                                let chunks = package.data.chunks(byte_in_frame);
                                let last_chunk_index = chunks.len() - 1;
                                for (index, ip_data) in chunks.enumerate() {
                                    let package = RedundancyPackage::new(ip_data.iter().cloned(),ip_data.len(),index != last_chunk_index,src_address,dest_address);
                                    redundancy.send(package).await;
                                }
                            }
                        }
                    },
                    package = redundancy.receive() =>{
                        trace!("fragment:{:?},len:{}",package,package.len());
                        let len = package.len();
                        let more_fragments = package.has_more_fragments();
                        let address = package.address();
                        data.extend(package.data.into_iter().skip(BYTE_IN_LENGTH + BYTE_IN_ENDING + BYTE_IN_ADDRESS).take(len));
                        trace!("merged_data:{:?}",data);
                        if !more_fragments {
                            let empty_data = Vec::new();
                            let data = std::mem::replace(&mut data,empty_data);
                            if address.0 != src_address{
                                recv_package_sender.send(IPPackage { data }).await;
                            }
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
    async fn send_raw(&self, package: IPPackage) {
        log::trace!("raw ip package len:{}",package.data.len());
        self.send_package_sender.send(package).await;
    }

    async fn receive_raw(&self) -> IPPackage {
        let mut guard = self.recv_package_receiver.lock().await;
        let package = guard.recv().await.unwrap();
        package
    }
}

impl IPLayer {
    pub async fn send<T>(&self, package: &T) where T: Decode + Encode {
        let encoded_package = bincode::encode_to_vec(&package, Configuration::standard()).unwrap();
        self.send_raw(IPPackage::new(encoded_package)).await;
    }

    pub async fn receive<T>(&self) -> Option<T> where T: Decode + Encode {
        let IPPackage { data } = self.receive_raw().await;
        Some(bincode::decode_from_slice(&data, Configuration::standard()).unwrap())
    }
}