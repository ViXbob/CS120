use std::sync::Arc;
use cs140_common::descriptor::SoundDescriptor;
use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use cs140_common::buffer::Buffer;

type DefaultBuffer = RingBuffer<f32, 5000000>;

pub struct PhysicalLayer {
    input_descriptor: SoundDescriptor,
    input_buffer: Arc<DefaultBuffer>,
    output_descriptor: SoundDescriptor,
    output_buffer: Arc<DefaultBuffer>,
    padding_noise_byte_len: usize,
    padding_zero_byte_len: usize,
}

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

use async_trait::async_trait;
use log::trace;
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::device::{InputDevice, OutputDevice};
use cs140_common::padding::padding_range;
use crate::framing::frame;

impl PhysicalLayer{
    pub fn new(padding_noise_byte_len:usize,padding_zero_byte_len:usize) -> Self {
        let input_buffer = Arc::new(DefaultBuffer::new());
        let (input_device, input_descriptor) = InputDevice::new(input_buffer.clone());
        let output_buffer = Arc::new(DefaultBuffer::new());
        let (output_device, output_descriptor) = OutputDevice::new(output_buffer.clone());
        // input_device.listen();
        output_device.play();
        PhysicalLayer {
            input_descriptor,
            input_buffer,
            output_descriptor,
            output_buffer,
            padding_noise_byte_len,
            padding_zero_byte_len
        }
    }
}

#[async_trait]
impl HandlePackage<PhysicalPackage> for PhysicalLayer {
    async fn send(&mut self, package: PhysicalPackage) {
        let mut samples = frame::generate_frame_sample_test(
            &package.0,
        );
        samples.extend(padding_range(-1.0,1.0).take(self.padding_noise_byte_len*8/2));
        samples.extend(std::iter::repeat(0.0).take(self.padding_zero_byte_len*8));
        self.output_buffer.push_by_ref(&samples).await;
    }

    async fn receive(&mut self) -> PhysicalPackage {
        loop {
            todo!()
        }
    }
}

#[cfg(test)]
mod tests{
    use cs140_common::padding::padding_inclusive_range;
    use super::*;

    #[tokio::test]
    async fn test_send_package(){
        let data:Vec<u8> = padding_inclusive_range(0..=255).take(6).collect();
        let data = BitStore::from_vec(data);
        let mut layer = PhysicalLayer::new(1, 1);
        layer.send(PhysicalPackage{
            0: data
        }).await;
        tokio::time::sleep(std::time::Duration::from_secs(10)).await;
    }
}