use std::sync::Arc;
use cs140_common::descriptor::SoundDescriptor;
use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use cs140_common::buffer::Buffer;

type DefaultBuffer = RingBuffer<f32, 5000000>;

pub struct PhysicalLayer {
    // input_descriptor: SoundDescriptor,
    pub input_buffer: Arc<DefaultBuffer>,
    pub output_descriptor: SoundDescriptor,
    output_buffer: Arc<DefaultBuffer>,
    padding_noise_byte_len: usize,
    padding_zero_byte_len: usize,
    max_package_byte_len: usize,
    zero_reader: ZeroReader,
}

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

use async_trait::async_trait;
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::device::{InputDevice, OutputDevice};
use crate::framing::frame;
use crate::sample_reader::{SampleReader, ZeroReader};

impl PhysicalLayer {
    pub fn new(padding_noise_byte_len: usize, padding_zero_byte_len: usize, max_package_byte_len: usize) -> Self {
        let input_buffer = Arc::new(DefaultBuffer::new());
        let (input_device, input_descriptor) = InputDevice::new_with_specific_device(input_buffer.clone(), 0);
        // std::thread::sleep(std::time::Duration::from_secs(33333));
        let output_buffer = Arc::new(DefaultBuffer::new());
        let (output_device, output_descriptor) = OutputDevice::new_with_specific_device(output_buffer.clone(), 0);
        input_device.listen();
        output_device.play();
        PhysicalLayer {
            // input_descriptor,
            input_buffer,
            output_descriptor,
            output_buffer,
            padding_noise_byte_len,
            padding_zero_byte_len,
            max_package_byte_len,
            zero_reader: ZeroReader::new(),
        }
    }
}

#[async_trait]
impl HandlePackage<PhysicalPackage> for PhysicalLayer {
    async fn send(&mut self, package: PhysicalPackage) {
        let mut samples = frame::generate_frame_sample_test(
            &package.0,
        );
        samples.extend(std::iter::repeat(0.0).take(self.padding_zero_byte_len * 8));
        self.output_buffer.push_by_ref(&samples).await;
    }

    async fn receive(&mut self) -> PhysicalPackage {
        loop {
            let something_more = 8;
            let max_sample_in_package = (self.max_package_byte_len + self.padding_zero_byte_len + something_more) * 8;
            let return_package = self.input_buffer.pop_by_ref(max_sample_in_package * 2, |data| {
                let index = self.zero_reader.read_all(data);
                return if index > max_sample_in_package {
                    (None, index - max_sample_in_package + something_more * 8)
                } else {
                    let data = &data[index..];
                    let mut sample_reader = SampleReader::from(self.zero_reader.clone());
                    let (bit_store, sample_used) = sample_reader.read_all(data);
                    self.zero_reader = sample_reader.into();
                    (Some(bit_store), sample_used + index)
                };
            }).await;
            if let Some(return_package) = return_package {
                return PhysicalPackage {
                    0: return_package
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use cs140_common::padding::padding_inclusive_range;
    use super::*;

    #[tokio::test]
    async fn test_send_package() {
        let data: Vec<u8> = padding_inclusive_range(0..=255).take(6).collect();
        let data = BitStore::from_vec(data);
        let mut layer = PhysicalLayer::new(1, 1);
        layer.send(PhysicalPackage {
            0: data
        }).await;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}