use std::sync::Arc;

use async_trait::async_trait;
use cpal::traits::{DeviceTrait, HostTrait};

use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::descriptor::SoundDescriptor;
use cs140_common::device::{InputDevice, OutputDevice};

use crate::encoding::{BitStore, decode_4b5b, decode_nrzi, encode_4b5b, encode_nrzi, HandlePackageMut, NetworkPackage};
use crate::sample_reader::{SampleReader, ZeroReader};

type DefaultBuffer = RingBuffer<f32, 5000000>;

pub struct PhysicalLayer {
    input_descriptor: SoundDescriptor,
    input_buffer: Arc<DefaultBuffer>,
    output_descriptor: SoundDescriptor,
    output_buffer: Arc<DefaultBuffer>,
    padding_zero_byte_len: usize,
    max_package_byte_len: usize,
    zero_reader: ZeroReader,
}

pub struct PhysicalPackage(BitStore);

impl PhysicalPackage {
    fn to_samples(&self) -> BitStore {
        let bits = &self.0;
        let bits = encode_4b5b(bits);
        let bits = encode_nrzi(&bits);
        bits
    }

    fn from_bits(bits: &BitStore) -> Self {
        let bits = decode_nrzi(bits);
        let bits = decode_4b5b(&bits);
        PhysicalPackage(bits)
    }
}

impl From<PhysicalPackage> for BitStore {
    fn from(package: PhysicalPackage) -> Self {
        package.0
    }
}

impl From<BitStore> for PhysicalPackage {
    fn from(bits: BitStore) -> Self {
        PhysicalPackage(bits)
    }
}

impl NetworkPackage for PhysicalPackage {}

impl PhysicalLayer {
    pub fn new(padding_zero_byte_len: usize, max_package_byte_len: usize) -> Self {
        let host = cpal::default_host();
        for (index, input_) in host.input_devices().unwrap().enumerate() {
            println!("input_device {}: {}", index, input_.name().unwrap());
        }
        println!("please choose your input audio device: ");
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf).unwrap();
        let input = buf.trim().parse().unwrap();
        let input_buffer = Arc::new(DefaultBuffer::new());
        let (input_device, input_descriptor) = InputDevice::new_with_specific_device(input_buffer.clone(), input);

        for (index, output_) in host.output_devices().unwrap().enumerate() {
            println!("output_device {}: {}", index, output_.name().unwrap());
        }
        println!("please choose your output audio device: ");
        buf.clear();
        std::io::stdin().read_line(&mut buf).unwrap();
        let output = buf.trim().parse().unwrap();
        let output_buffer = Arc::new(DefaultBuffer::new());
        let (output_device, output_descriptor) = OutputDevice::new_with_specific_device(output_buffer.clone(), output);
        input_device.listen();
        output_device.play();
        PhysicalLayer {
            input_descriptor,
            input_buffer,
            output_descriptor,
            output_buffer,
            padding_zero_byte_len,
            max_package_byte_len,
            zero_reader: ZeroReader::new(),
        }
    }

    pub fn max_package_byte(&self) -> usize {
        self.max_package_byte_len
    }
}

#[async_trait]
impl HandlePackageMut<PhysicalPackage> for PhysicalLayer {
    async fn send(&mut self, package: PhysicalPackage) {
        let mut samples: Vec<_> = package.to_samples().into_iter().flat_map(|bit| {
            if bit {
                std::iter::repeat(1.0).take(2)
            } else {
                std::iter::repeat(-1.0).take(2)
            }
        }).collect();
        samples.extend(std::iter::repeat(0.0).take(self.padding_zero_byte_len * 8));
        self.output_buffer.push_by_ref(&samples).await;
    }

    async fn receive(&mut self) -> PhysicalPackage {
        loop {
            let something_more = 7;
            let margin = (self.padding_zero_byte_len + something_more) * 8;
            let max_sample_in_package = self.max_package_byte_len * 8 / 4 * 5 * 2 + margin;
            let return_package = self.input_buffer.pop_by_ref(max_sample_in_package + margin, |data| {
                let index = self.zero_reader.read_all(data);
                return if index > margin {
                    (None, index)
                } else {
                    let data = &data[index..];
                    let mut sample_reader = SampleReader::from(self.zero_reader.clone());
                    let (bit_store, sample_used) = sample_reader.read_all(data);
                    self.zero_reader = sample_reader.into();
                    (Some(bit_store), sample_used + index)
                };
            }).await;
            if let Some(return_package) = return_package {
                return PhysicalPackage::from_bits(&return_package);
            }
        }
    }
}

#[cfg(test)]
mod tests {}