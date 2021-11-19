use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::framing::frame;
use crate::framing::header::create_header;
use async_trait::async_trait;
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::descriptor::SoundDescriptor;
use cs140_common::device::{InputDevice, OutputDevice};
use cs140_common::padding::padding_range;
use std::sync::Arc;

type DefaultBuffer = RingBuffer<f32, 5000000>;

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

const HEADER_LENGTH: usize = 60;
const MIN_FREQUENCY: f32 = 8000.0;
const MAX_FREQUENCY: f32 = 11000.0;
const SPEED: u32 = 1000;
const TIME_OUT: u32 = 30;
const SPEED_OF_PSK: u32 = 12000;

// a frame in physical layer has #(frame_length * sample_per_bit) samples

pub struct PhysicalLayer {
    input_descriptor: SoundDescriptor,
    input_buffer: Arc<DefaultBuffer>,
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
            &padding_range(-0.1, 0.1)
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

    pub fn new_with_specific_device(multiplex_frequency: &[f32], byte_in_frame: usize, device_name: usize) -> Self {
        let input_buffer = Arc::new(DefaultBuffer::new());
        let (input_device, input_descriptor) = InputDevice::new_with_specific_device(input_buffer.clone(), device_name);
        let output_buffer = Arc::new(DefaultBuffer::new());
        let (output_device, output_descriptor) = OutputDevice::new_with_specific_device(output_buffer.clone(), device_name);
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
        let _ = input_device.listen();
        let _ = output_device.play();
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

#[async_trait]
impl HandlePackage<PhysicalPackage> for PhysicalLayer {
    async fn send(&mut self, package: PhysicalPackage) {
        let samples = frame::generate_frame_sample_from_bitvec(
            &package.0,
            &self.header,
            &self.multiplex_frequency,
            self.output_descriptor.sample_rate,
            self.speed,
        );
        self.output_buffer.push_by_ref(&samples).await;
        let noise = padding_range(-0.1, 0.1)
            .take(HEADER_LENGTH)
            .collect::<Vec<f32>>();
        self.output_buffer.push_by_ref(
            &noise,
        ).await;
    }

    async fn receive(&mut self) -> PhysicalPackage {
        loop {
            let return_package = self
                .input_buffer
                .pop_by_ref(
                    2 * self.frame_length * self.input_descriptor.sample_rate as usize
                        / self.speed as usize,
                    |data| {
                        frame::frame_resolve_to_bitvec(
                            data,
                            &self.header,
                            &self.multiplex_frequency,
                            self.input_descriptor.sample_rate,
                            self.speed,
                            self.frame_length,
                        )
                    },
                )
                .await;
            if return_package.is_some() {
                return PhysicalPackage(return_package.unwrap());
            }
        }
    }
}
