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
    byte_in_frame: usize,
}

impl IPLayer {
    pub fn new(redundancy: RedundancyLayer) -> Self {
        let byte_in_frame = redundancy.byte_in_frame - 3;
        IPLayer {
            redundancy,
            byte_in_frame,
        }
    }
}

impl HandlePackage<IPPackage> for IPLayer {
    fn send(&mut self, package: IPPackage) {
        let chunks = package.data.chunks(self.byte_in_frame);
        let last_chunk_index = chunks.len() - 1;
        for (index, ip_data) in chunks.enumerate() {
            let mut data = Vec::with_capacity(self.redundancy.byte_in_frame);
            let len = ip_data.len() as u16;
            data.push(((len & 0xff00) >> 8) as u8);
            data.push((len & 0x00ff) as u8);
            data.push(if index == last_chunk_index {
                0b00000001
            } else {
                0
            });
            data.extend(ip_data.into_iter());
            data.resize(self.redundancy.byte_in_frame, 0);
            self.redundancy.send(RedundancyPackage { data });
            println!("Package {} sent, len {}.", index, len)
        }
    }

    fn receive(&mut self) -> IPPackage {
        let mut data: Vec<u8> = Vec::new();
        loop {
            let package: RedundancyPackage = self.redundancy.receive();
            let len = ((package.data[0] as usize) << 8) + package.data[1] as usize;
            // println!("we received a package with len:{}", len);
            let ended = (package.data[2] & 1) == 1;
            // if ended{
            //     println!("the package is ended");
            // }
            data.extend(package.data.into_iter().skip(3).take(len));
            if ended {
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

#[cfg(test)]
mod test {
    use crate::encoding::HandlePackage;
    use crate::framing::frame;
    use crate::ip::{IPLayer, IPPackage};
    use crate::physical::PhysicalLayer;
    use crate::redundancy::RedundancyLayer;
    use bitvec::order::Lsb0;
    use bitvec::vec::BitVec;
    use cs140_common::buffer::Buffer;
    use rand::Rng;

    const FREQUENCY: &'static [f32] = &[3000.0, 6000.0];
    const BYTE_PER_FRAME: usize = 128;
    fn generate_data(
        size: usize,
        header: &Vec<f32>,
        multiplex_frequency: &[f32],
    ) -> (Vec<f32>, BitVec<Lsb0, u8>) {
        let mut data: BitVec<Lsb0, u8> = BitVec::new();
        for i in 0..size {
            data.push(rand::thread_rng().gen::<bool>());
        }
        let mut samples = frame::generate_frame_sample_from_bitvec(
            &data,
            header,
            multiplex_frequency,
            48000,
            1000,
        );
        (samples, data)
    }

    fn push_data_to_buffer<T: Buffer<f32>>(
        buffer: &T,
        ip: &mut IPLayer,
        size: usize,
        frame_size: usize,
        header: &Vec<f32>,
        multiplex_frequency: &[f32],
    ) -> BitVec<Lsb0, u8> {
        buffer.push_by_iterator(
            30000,
            &mut (0..30000)
                .map(|x| (x as f32 * 6.28 * 3000.0 / 48000.0).sin() * 0.5)
                .take(30000),
        );
        let mut data = BitVec::new();
        let (samples, data_) = generate_data(size, header, multiplex_frequency);
        // buffer.push_by_ref(&samples);
        ip.send(IPPackage {
            data: data.clone().into_vec(),
        });
        buffer.push_by_iterator(10000, &mut std::iter::repeat(0.0));
        data
    }

    fn test_ip_layer() {
        let physical: PhysicalLayer = PhysicalLayer::new(FREQUENCY, BYTE_PER_FRAME * 2);
        let header = physical.header.clone();
        let output_buffer = physical.output_buffer.clone();
        let redundancy: RedundancyLayer = RedundancyLayer::new(physical);
        let mut ip_server: IPLayer = IPLayer::new(redundancy);
        let ground_truth = push_data_to_buffer(
            &*output_buffer,
            &mut ip_server,
            10000,
            BYTE_PER_FRAME * 8,
            &header,
            FREQUENCY,
        );
        // ip_server.receive();
    }
}
