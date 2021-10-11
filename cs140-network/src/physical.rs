use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::framing::frame;
use crate::framing::header;
use bitvec::vec::BitVec;
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::device::{InputDevice, OutputDevice};
use std::sync::Arc;

type DefaultBuffer = RingBuffer<f32, 100000, false>;

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

pub struct PhysicalLayer {
    input: InputDevice<DefaultBuffer>,
    output: OutputDevice<DefaultBuffer>,
    buffer: Arc<DefaultBuffer>,
    multiplex_frequency: &'static [f32],
    header: [f32; 220],
    speed: u32,
    frame_length: usize,
}

impl PhysicalLayer{
    fn new(multiplex_frequency:&[f32])->Self{
        unimplemented!();
        let buffer = Arc::new(DefaultBuffer::new());
        let (input_device, input_descriptor) = InputDevice::new(buffer.clone());
        let (output_device,output_descriptor) = OutputDevice::new(buffer.clone());
        PhysicalLayer{
            input: input_device,
            output: output_device,
            buffer,
            multiplex_frequency: &multiplex_frequency.to_owned(),
            header: [0.0;220],
            speed: 0,
            frame_length: 0
        }
    }
}

impl HandlePackage<PhysicalPackage> for PhysicalLayer {
    fn send(&mut self, package: PhysicalPackage) {
        let samples = frame::generate_frame_sample_from_bitvec(
            &package.0,
            self.multiplex_frequency.len(),
            &self.multiplex_frequency,
            self.output.sound_descriptor().sample_rate,
            self.speed,
        );
        let segment_len = 100;
        for segment in samples.chunks(segment_len) {
            // segment push
            self.buffer.push_by_ref(segment);
        }
    }

    fn receive(&mut self) -> PhysicalPackage {
        loop {
            let mut return_package = self.buffer.pop_by_ref(111, |data| {
                frame::frame_resolve_to_bitvec(
                    data,
                    self.header.len(),
                    &self.header,
                    &self.multiplex_frequency,
                    self.output.sound_descriptor().sample_rate,
                    self.speed,
                    self.frame_length,
                )
            });
            if return_package.is_some() {
                return PhysicalPackage(return_package.unwrap());
            }
        }
    }
}
