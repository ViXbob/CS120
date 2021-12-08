use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::framing::frame;
use crate::framing::header::create_header;
use crate::redundancy::RedundancyPackage;
use crate::ack::ack::AckPackage;
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::descriptor::SoundDescriptor;
use cs140_common::device::{InputDevice, OutputDevice};
use cs140_common::padding::{padding, padding_range};
use std::sync::Arc;

type DefaultBuffer = RingBuffer<f32, 5000000>;

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

const HEADER_LENGTH: usize = 60;
const MIN_FREQUENCY: f32 = 8000.0;
const MAX_FREQUENCY: f32 = 11000.0;
const SAMPLE_PER_BIT: usize = 4;

// a frame in physical layer has #(frame_length * sample_per_bit) samples

pub struct PhysicalLayer {
    input_descriptor: SoundDescriptor,
    pub(crate) input_buffer: Arc<DefaultBuffer>,
    output_descriptor: SoundDescriptor,
    pub(crate) output_buffer: Arc<DefaultBuffer>,
    pub(crate) frame_length: usize,
    pub(crate) header: Vec<f32>,
    pub(crate) byte_in_frame: usize,
    pub(crate) sample_per_bit: usize,
}

impl PhysicalLayer {
    fn push_warm_up_data_to_buffer(buffer: &Arc<DefaultBuffer>, time: usize) {
        buffer.push_by_ref(
            &padding_range(-0.01, 0.01)
                .take(48 * time)
                .collect::<Vec<f32>>(),
        );
    }

    pub fn push_warm_up_data(&self, time: usize) {
        Self::push_warm_up_data_to_buffer(&self.output_buffer, time);
    }

    pub fn new(multiplex_frequency: &[f32], byte_in_frame: usize) -> Self {
        let input_buffer = Arc::new(DefaultBuffer::new());
        let (input_device, input_descriptor) = InputDevice::new(input_buffer.clone());
        let output_buffer = Arc::new(DefaultBuffer::new());
        let (output_device, output_descriptor) = OutputDevice::new(output_buffer.clone());
        input_device.listen();
        output_device.play();
        let sample_rate = output_descriptor.sample_rate;
        PhysicalLayer {
            input_descriptor,
            input_buffer,
            output_descriptor,
            output_buffer,
            frame_length: byte_in_frame * 8,
            header: create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, sample_rate),
            byte_in_frame,
            sample_per_bit: SAMPLE_PER_BIT,
        }
    }

    pub fn new_with_specific_device(byte_in_frame: usize, input_device: usize, output_device: usize) -> Self {
        let input_buffer = Arc::new(DefaultBuffer::new());
        let (input_device, input_descriptor) = InputDevice::new_with_specific_device(input_buffer.clone(), input_device);
        let output_buffer = Arc::new(DefaultBuffer::new());
        let (output_device, output_descriptor) = OutputDevice::new_with_specific_device(output_buffer.clone(), output_device);
        input_device.listen();
        output_device.play();
        let sample_rate = output_descriptor.sample_rate;
        PhysicalLayer {
            input_descriptor,
            input_buffer,
            output_descriptor,
            output_buffer,
            frame_length: byte_in_frame * 8,
            header: create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, sample_rate),
            byte_in_frame,
            sample_per_bit: SAMPLE_PER_BIT,
        }
    }

    pub fn new_send_only(multiplex_frequency: &[f32], byte_in_frame: usize) -> Self {
        let input_buffer = Arc::new(DefaultBuffer::new());
        let (_, input_descriptor) = InputDevice::new(input_buffer.clone());
        let output_buffer = Arc::new(DefaultBuffer::new());
        let (output_device, output_descriptor) = OutputDevice::new(output_buffer.clone());
        Self::push_warm_up_data_to_buffer(&output_buffer, 10);
        output_device.play();
        let sample_rate = output_descriptor.sample_rate;
        PhysicalLayer {
            input_descriptor,
            input_buffer,
            output_descriptor,
            output_buffer,
            frame_length: byte_in_frame * 8,
            header: create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, sample_rate),
            byte_in_frame,
            sample_per_bit: SAMPLE_PER_BIT,
        }
    }

    pub fn new_receive_only(multiplex_frequency: &[f32], byte_in_frame: usize) -> Self {
        let input_buffer = Arc::new(DefaultBuffer::new());
        let (input_device, input_descriptor) = InputDevice::new(input_buffer.clone());
        let output_buffer = Arc::new(DefaultBuffer::new());
        let (_, output_descriptor) = OutputDevice::new(output_buffer.clone());
        input_device.listen();
        let sample_rate = output_descriptor.sample_rate;
        PhysicalLayer {
            input_descriptor,
            input_buffer,
            output_descriptor,
            output_buffer,
            frame_length: byte_in_frame * 8,
            header: create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, sample_rate),
            byte_in_frame,
            sample_per_bit: SAMPLE_PER_BIT,
        }
    }
}

use async_trait::async_trait;
use log::trace;

#[async_trait]
impl HandlePackage<PhysicalPackage> for PhysicalLayer {
    async fn send(&mut self, package: PhysicalPackage) {
        let mut samples = frame::generate_frame_sample_from_bitvec(
            &package.0,
            &self.header,
            self.sample_per_bit,
        );
        let noise = samples.iter().cloned().skip(samples.len() - 64)
            .collect::<Vec<f32>>();
        samples.extend(noise.into_iter());
        self.output_buffer.push_by_ref(&samples).await;
    }

    async fn receive(&mut self) -> PhysicalPackage {
        loop {
            let return_package = self.input_buffer.pop_by_ref(
                2 * self.frame_length * self.sample_per_bit,
                |data| {
                    let current = std::time::Instant::now();
                    frame::frame_resolve_to_bitvec(
                        data,
                        &self.header,
                        self.sample_per_bit,
                        self.frame_length,
                    )
                },
            ).await;
            if return_package.is_none() { continue; }
            if return_package.as_ref().unwrap().is_empty() { continue; }
            for package in return_package.as_ref().unwrap() {
                let physical_package = PhysicalPackage {
                    0: package.clone()
                };
                let redundancy_package = RedundancyPackage { data: package.clone().into_vec() };
                if redundancy_package.validate_checksum() {
                    return physical_package;
                }
            }
            trace!("{}", return_package.as_ref().unwrap()[0].len());
            trace!("{:?}", return_package.as_ref().unwrap()[0]);
            return PhysicalPackage {
                0: return_package.as_ref().unwrap()[0].clone()
            };
        }
    }
}