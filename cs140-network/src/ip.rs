use crate::encoding::{HandlePackage, NetworkPackage};
use crate::physical::PhysicalPackage;
use crate::redundancy::{BYTE_IN_ADDRESS, BYTE_IN_ENDING, BYTE_IN_LENGTH, RedundancyLayer, RedundancyPackage};
use cs140_common::padding;

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
    redundancy: RedundancyLayer,
    byte_in_frame: usize,
}

impl IPLayer {
    pub fn new(redundancy: RedundancyLayer) -> Self {
        let byte_in_frame = redundancy.byte_in_frame;
        IPLayer {
            redundancy,
            byte_in_frame,
        }
    }
}
use async_trait::async_trait;

#[async_trait]
impl HandlePackage<IPPackage> for IPLayer {
    async fn send(&mut self, package: IPPackage) {
        let chunks = package.data.chunks(self.byte_in_frame);
        let last_chunk_index = chunks.len() - 1;
        for (index, ip_data) in chunks.enumerate() {
            if index == last_chunk_index{
                let package = RedundancyPackage::new(ip_data.iter().cloned().chain(padding::padding()).take(self.redundancy.byte_in_frame),self.redundancy.byte_in_frame,false,0,0);
                self.redundancy.send(package).await;
            }else{
                self.redundancy.send(RedundancyPackage::new(ip_data.iter().cloned(),self.redundancy.byte_in_frame,true,0,0)).await;
            }
        }
    }

    async fn receive(&mut self) -> IPPackage {
        let mut data: Vec<u8> = Vec::new();
        // let mut package_received = 0;
        loop {
            let package: RedundancyPackage = self.redundancy.receive().await;
            // let len = ((package.data[0] as usize) << 8) + package.data[1] as usize;
            let len = package.data_len();
            // println!("we received a package with len:{}", len);
            // let ended = (package.data[2] & 1) == 1;
            let more_fragments = package.has_more_fragments();
            // if ended{
            //     println!("the package is ended");
            // }
            data.extend(package.data.into_iter().skip(BYTE_IN_LENGTH + BYTE_IN_ENDING + BYTE_IN_ADDRESS).take(len));
            // println!("package_received:{}, {:?}", package_received, data);
            if !more_fragments {
                return IPPackage { data };
            }
        }
    }
}

// impl HandlePackage<RedundancyPackage> for IPLayer {
//     fn send(&mut self, package: RedundancyPackage) {
//         self.redundancy.send(package)
//     }
//
//     fn receive(&mut self) -> RedundancyPackage {
//         self.redundancy.receive()
//     }
//
//     fn receive_time_out(&mut self) -> Option<RedundancyPackage> {
//         todo!()
//     }
// }
//
// impl HandlePackage<PhysicalPackage> for IPLayer {
//     fn send(&mut self, package: PhysicalPackage) {
//         self.redundancy.send(package)
//     }
//
//     fn receive(&mut self) -> PhysicalPackage {
//         self.redundancy.receive()
//     }
//
//     fn receive_time_out(&mut self) -> Option<PhysicalPackage> {
//         todo!()
//     }
// }
