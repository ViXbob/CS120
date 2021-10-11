use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::framing::frame;
use crate::framing::header;
use bitvec::vec::BitVec;
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::device::{InputDevice, OutputDevice};
use std::sync::Arc;
use cs140_common::descriptor::SoundDescriptor;
use crate::framing::header::create_header;

type DefaultBuffer = RingBuffer<f32, 100000, false>;

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

pub struct PhysicalLayer {
    input_descriptor: SoundDescriptor,
    input_buffer: Arc<DefaultBuffer>,
    output_descriptor: SoundDescriptor,
    output_buffer: Arc<DefaultBuffer>,
    multiplex_frequency: Vec<f32>,
    speed: u32,
    frame_length: usize,
    header: Vec<f32>,
}

impl PhysicalLayer {
    fn new(multiplex_frequency: &[f32], frame_length: usize) -> Self {
        let input_buffer = Arc::new(DefaultBuffer::new());
        let (input_device, input_descriptor) = InputDevice::new(input_buffer.clone());
        let output_buffer = Arc::new(DefaultBuffer::new());
        let (output_device, output_descriptor) = OutputDevice::new(output_buffer.clone());
        input_device.listen();
        output_device.play();
        PhysicalLayer {
            input_descriptor,
            input_buffer,
            output_descriptor,
            output_buffer,
            multiplex_frequency: multiplex_frequency.to_owned(),
            speed: 1000,
            frame_length: frame_length,
            header: create_header(220, 3000.0, 6000.0, 48000),
        }
    }
}

impl HandlePackage<PhysicalPackage> for PhysicalLayer {
    fn send(&mut self, package: PhysicalPackage) {
        let samples = frame::generate_frame_sample_from_bitvec(
            &package.0,
            &self.header,
            &self.multiplex_frequency,
            self.output_descriptor.sample_rate,
            self.speed,
        );
        let segment_len = 100;
        for segment in samples.chunks(segment_len) {
            // segment push
            self.output_buffer.push_by_ref(segment);
        }
    }

    fn receive(&mut self) -> PhysicalPackage {
        loop {
            let mut return_package = self.input_buffer.pop_by_ref(2 * self.frame_length * self.input_descriptor.sample_rate as usize / self.speed as usize, |data| {
                frame::frame_resolve_to_bitvec(
                    data,
                    &self.header,
                    &self.multiplex_frequency,
                    self.input_descriptor.sample_rate,
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

#[cfg(test)]
mod test {
    use super::*;
    use bitvec::prelude::*;
    use rand::Rng;
    use rand::seq::index::sample;
    use cs140_common::buffer::Buffer;

    fn generate_data(size: usize, header: &Vec<f32>, multiplex_frequency: &[f32]) -> (Vec<f32>, BitVec<Lsb0, u8>) {
        let mut data: BitVec<Lsb0, u8> = BitVec::new();
        for i in 0..size {
            data.push(rand::thread_rng().gen::<bool>());
        }
        let samples = frame::generate_frame_sample_from_bitvec(
            &data,
            header,
            multiplex_frequency,
            48000,
            1000,
        );
        (samples,data)
    }

    fn push_data_to_buffer<T: Buffer<f32>>(buffer: &T, size: usize, header: &Vec<f32>, multiplex_frequency: &[f32]) -> BitVec<Lsb0, u8> {
        let (samples, data) = generate_data(size, header, multiplex_frequency);
        buffer.push_by_iterator(30000, &mut (0..30000).map(|x| (x as f32 * 6.28 * 3000.0 / 48000.0).sin() * 0.0).take(30000));
        buffer.push_by_ref(&samples);
        buffer.push_by_iterator(10000, &mut std::iter::repeat(0.0));
        data
    }

    #[test]
    fn test_decode_frame() {
        const SIZE:usize = 512;
        const FREQUENCY:&'static [f32] = &[3000.0,6000.0];
        let header = create_header(220, 3000.0, 6000.0, 48000);
        let buffer = DefaultBuffer::new();
        let ground_truth = push_data_to_buffer(&buffer, SIZE,&header,FREQUENCY);
        let result = buffer.pop_by_ref(25000, |data| {
            frame::frame_resolve_to_bitvec(
                data,
                &header,
                &[5000.0],
                48000,
                1000,
                512,
            )
        });
        let result = result.unwrap();
        println!("result:\t{:?}", result);
        println!("source:\t{:?}", ground_truth);
        assert_eq!(result, ground_truth)
    }

    #[test]
    fn test_decode_frame_from_physical_layer() {
        const SIZE:usize = 512;
        const FREQUENCY:&'static [f32] = &[3000.0,6000.0];
        let mut layer = PhysicalLayer::new(FREQUENCY, SIZE/FREQUENCY.len());
        let header = layer.header.clone();
        let output_buffer = layer.output_buffer.clone();
        let handle = std::thread::spawn(move || {
            push_data_to_buffer(&*output_buffer, SIZE,&header,FREQUENCY)
        });
        let ground_truth = handle.join().unwrap();
        let response = layer.receive().0;
        println!("result:\t{:?}", response);
        println!("source:\t{:?}", ground_truth);
        assert_eq!(response, ground_truth)
    }
}
