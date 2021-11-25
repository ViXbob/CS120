use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::framing::frame;
use crate::framing::header::create_header;
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
const SPEED: u32 = 1000;
const SPEED_OF_PSK: u32 = 12000;

// a frame in physical layer has #(frame_length * sample_per_bit) samples

pub struct PhysicalLayer {
    input_descriptor: SoundDescriptor,
    pub(crate) input_buffer: Arc<DefaultBuffer>,
    output_descriptor: SoundDescriptor,
    pub(crate) output_buffer: Arc<DefaultBuffer>,
    multiplex_frequency: Vec<f32>,
    speed: u32,
    pub(crate) frame_length: usize,
    pub(crate) header: Vec<f32>,
    pub(crate) byte_in_frame: usize,
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
            multiplex_frequency: multiplex_frequency.to_owned(),
            speed: SPEED,
            frame_length: byte_in_frame * 8 / multiplex_frequency.len(),
            header: create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, sample_rate),
            byte_in_frame,
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
            multiplex_frequency: multiplex_frequency.to_owned(),
            speed: SPEED,
            frame_length: byte_in_frame * 8 / multiplex_frequency.len(),
            header: create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, sample_rate),
            byte_in_frame,
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
            multiplex_frequency: multiplex_frequency.to_owned(),
            speed: SPEED,
            frame_length: byte_in_frame * 8 / multiplex_frequency.len(),
            header: create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, sample_rate),
            byte_in_frame,
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
            multiplex_frequency: multiplex_frequency.to_owned(),
            speed: SPEED,
            frame_length: byte_in_frame * 8 / multiplex_frequency.len(),
            header: create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, sample_rate),
            byte_in_frame,
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
                2 * self.frame_length * self.input_descriptor.sample_rate as usize
                    / self.speed as usize,
                |data| {
                    let current = std::time::Instant::now();
                    frame::frame_resolve_to_bitvec(
                        data,
                        &self.header,
                        &self.multiplex_frequency,
                        self.input_descriptor.sample_rate,
                        self.speed,
                        self.frame_length,
                    )
                },
            ).await;
            if let Some(package) = return_package {
                return PhysicalPackage {
                    0: package
                };
            }
        }
    }
}