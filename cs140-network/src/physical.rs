use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::framing::frame;
use crate::framing::header::create_header;
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::descriptor::SoundDescriptor;
use cs140_common::device::{InputDevice, OutputDevice};
use std::sync::Arc;

type DefaultBuffer = RingBuffer<f32, 1500000, false>;

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

const HEADER_LENGTH: usize = 220;
const MIN_FREQUENCY: f32 = 3000.0;
const MAX_FREQUENCY: f32 = 6000.0;

pub struct PhysicalLayer {
    input_descriptor: SoundDescriptor,
    input_buffer: Arc<DefaultBuffer>,
    output_descriptor: SoundDescriptor,
    output_buffer: Arc<DefaultBuffer>,
    multiplex_frequency: Vec<f32>,
    speed: u32,
    pub(crate) frame_length: usize,
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
            frame_length,
            header: create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, 48000),
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
            let return_package = self.input_buffer.pop_by_ref(
                2 * self.frame_length * self.input_descriptor.sample_rate as usize
                    / self.speed as usize,
                |data| {
                    let tmp = frame::frame_resolve_to_bitvec(
                        data,
                        &self.header,
                        &self.multiplex_frequency,
                        self.input_descriptor.sample_rate,
                        self.speed,
                        self.frame_length,
                    );
                    println!("begin_index = {}", tmp.1);
                    tmp
                },
            );
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
    use cs140_common::buffer::Buffer;
    use rand::seq::index::sample;
    use rand::Rng;

    fn generate_data(
        size: usize,
        header: &Vec<f32>,
        multiplex_frequency: &[f32],
    ) -> (Vec<f32>, BitVec<Lsb0, u8>) {
        let mut data: BitVec<Lsb0, u8> = BitVec::new();
        for i in 0..size {
            data.push(rand::thread_rng().gen::<bool>());
        }
        let mut samples = frame::generate_frame_sample_from_bitvec(
            &data,
            header,
            multiplex_frequency,
            48000,
            1000,
        );
        (samples, data)
    }

    fn push_data_to_buffer<T: Buffer<f32>>(
        buffer: &T,
        size: usize,
        frame_size: usize,
        header: &Vec<f32>,
        multiplex_frequency: &[f32],
    ) -> BitVec<Lsb0, u8> {
        buffer.push_by_iterator(
            30000,
            &mut (0..30000)
                .map(|x| (x as f32 * 6.28 * 3000.0 / 48000.0).sin() * 0.5)
                .take(30000),
        );
        let mut data = BitVec::new();
        for i in 0..frame_size {
            let (samples, data_) = generate_data(size, header, multiplex_frequency);
            buffer.push_by_ref(&samples);
            data.extend(data_.iter());
        }
        buffer.push_by_iterator(10000, &mut std::iter::repeat(0.0));
        data
    }

    #[test]
    fn test_decode_frame() {
        const SIZE: usize = 512;
        const FREQUENCY: &'static [f32] = &[3000.0, 6000.0];
        let header = create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, 48000);
        let buffer = DefaultBuffer::new();
        let ground_truth = push_data_to_buffer(&buffer, SIZE, 1, &header, FREQUENCY);
        let result = buffer.pop_by_ref(25000, |data| {
            frame::frame_resolve_to_bitvec(data, &header, &[5000.0], 48000, 1000, 512)
        });
        let result = result.unwrap();
        println!("result:\t{:?}", result);
        println!("source:\t{:?}", ground_truth);
        assert_eq!(result, ground_truth)
    }

    #[test]
    fn test_decode_frame_from_physical_layer() {
        const SIZE: usize = 10000;
        const FREQUENCY: &'static [f32] = &[4000.0, 5000.0];
        const FRAME_SIZE: usize = 1;
        let mut layer = PhysicalLayer::new(FREQUENCY, SIZE / FREQUENCY.len());
        let header = layer.header.clone();
        let output_buffer = layer.output_buffer.clone();
        let handle = std::thread::spawn(move || {
            push_data_to_buffer(&*output_buffer, SIZE, FRAME_SIZE, &header, FREQUENCY)
        });
        let ground_truth = handle.join().unwrap();
        let mut response: super::BitStore = BitVec::new();
        for _ in 0..FRAME_SIZE {
            response.extend(layer.receive().0.iter());
        }
        println!("result:\t{:?}", response);
        println!("source:\t{:?}", ground_truth);
        let mut errors = 0;
        for (a, b) in response.iter().zip(ground_truth.clone()) {
            if a != b {
                errors += 1;
            }
        }
        println!("{}", errors);
        assert_eq!(response, ground_truth)
    }

    #[test]
    fn record_samples() {
        let size = 512;
        let header = create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, 48000);
        let multiplex_frequency: &[f32] = &[4000.0, 5000.0];
        let mut tmp: Vec<_> = (0..30000)
            .map(|x| (x as f32 * 6.28 * 3000.0 / 48000.0).sin() * 0.5)
            .take(30000)
            .collect();
        for _ in 0..35 {
            let (samples, _) = generate_data(size, &header, multiplex_frequency);
            tmp.extend(samples.iter());
        }
        let tmp_: Vec<_> = std::iter::repeat(0.0).take(10000).collect();
        tmp.extend(tmp_.iter());
        // cs140_util::record::record_from_slice("/Users/vixbob/cs140/output.wav",tmp.as_slice());
        let buffer: RingBuffer<f32, 100000, false> = RingBuffer::new();
        let buffer_ptr = Arc::new(buffer);
        let (output, descriptor) = OutputDevice::new(buffer_ptr.clone());
        let close_output = output.play();
        for samples in tmp.chunks(100) {
            buffer_ptr.push_by_ref(samples);
        }
        std::thread::sleep(std::time::Duration::from_secs(12));
        close_output();
    }
}
