use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::device::{InputDevice, OutputDevice};
use crate::header;
use crate::frame;
use std::sync::Arc;
use bitvec::vec::BitVec;

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

pub struct PhysicalLayer {
    input: InputDevice<RingBuffer<f32, 100000, false> >,
    output: OutputDevice<RingBuffer<f32, 100000, false> >,
    buffer_ptr: Arc<RingBuffer<f32, 100000, false> >,
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
            self.speed);
        let segment_len = 100;
        std::thread::spawn(move || {
            for segment in samples.iter().chunks(segment_len) {
                // segment push
                self.buffer_ptr.push_by_ref(segment);
            }
        });
    }

    fn receive(&mut self) -> PhysicalPackage {
        let mut return_package : Option<BitStore> = Some(BitVec::new());
        loop {
            self.buffer_ptr.push(111, |data| -> usize {
                let tmp = frame::frame_resolve_to_bitvec(data,
                                                         self.header.len(),
                                                         &self.header,
                                                         &self.multiplex_frequency,
                                                         self.output.sound_descriptor().sample_rate,
                                                         self.speed, self.frame_length);
                return_package = tmp.0;
                return tmp.1;
            });
            if return_package.is_some() { break; }
        }
        PhysicalPackage(return_package.unwrap())
    }
}
