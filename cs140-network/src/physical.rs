use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::device::{InputDevice, OutputDevice};

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

pub struct PhysicalLayer {
    input: InputDevice<RingBuffer<f32, 100000, false> >,
    output: OutputDevice<RingBuffer<f32, 100000, false> >,
}

impl HandlePackage<PhysicalPackage> for PhysicalLayer {
    fn send(&mut self, package: PhysicalPackage) {
        todo!()
    }

    fn receive(&mut self) -> PhysicalPackage {
        todo!()
    }
}
