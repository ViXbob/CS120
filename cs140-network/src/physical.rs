use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::framing::frame;
use crate::framing::header;
use bitvec::vec::BitVec;
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::device::{InputDevice, OutputDevice};
use std::sync::Arc;

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

type DefaultBuffer = RingBuffer<f32, 100000, false>;

pub struct PhysicalLayer {
    input: InputDevice<DefaultBuffer>,
    output: OutputDevice<DefaultBuffer>,
    buffer_ptr: Arc<DefaultBuffer>,
    multiplex_frequency: [f32; 1],
    header: [f32; 220],
    speed: u32,
    frame_length: usize,
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
            self.buffer_ptr.push_by_ref(segment);
        }
    }

    fn receive(&mut self) -> PhysicalPackage {
        loop {
            let mut return_package = self.buffer_ptr.pop_by_ref(111, |data| {
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
