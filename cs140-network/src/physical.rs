use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use crate::framing::frame;
use crate::framing::header::create_header;
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::descriptor::SoundDescriptor;
use cs140_common::device::{InputDevice, OutputDevice};
use cs140_common::padding::{padding, padding_range};
use std::sync::Arc;

type DefaultBuffer = RingBuffer<f32, 5000000, false>;

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

const HEADER_LENGTH: usize = 60;
const MIN_FREQUENCY: f32 = 5000.0;
const MAX_FREQUENCY: f32 = 8000.0;
const SPEED: u32 = 1000;
const TIME_OUT: u32 = 30;

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
    fn push_warm_up_data_to_buffer(buffer: &Arc<DefaultBuffer>) {
        buffer.push_by_ref(
            &padding_range(-0.1, 0.1)
                .take(HEADER_LENGTH)
                .collect::<Vec<f32>>(),
        );
    }

    pub fn push_warm_up_data(&self) {
        Self::push_warm_up_data_to_buffer(&self.output_buffer);
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
        Self::push_warm_up_data_to_buffer(&output_buffer);
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

impl HandlePackage<PhysicalPackage> for PhysicalLayer {
    fn send(&mut self, package: PhysicalPackage) {
        let samples = frame::generate_frame_sample_from_bitvec(
            &package.0,
            &self.header,
            &self.multiplex_frequency,
            self.output_descriptor.sample_rate,
            self.speed,
        );
        self.output_buffer.push_by_ref(
            &padding_range(-0.05, 0.05)
                .take(30)
                .collect::<Vec<f32>>(),
        );
        let segment_len = 100;
        for segment in samples.chunks(segment_len) {
            // segment push
            self.output_buffer.push_by_ref(segment);
        }
        self.output_buffer.push_by_ref(
            &padding_range(-0.1, 0.1)
                .take(30)
                .collect::<Vec<f32>>(),
        );
    }

    fn receive(&mut self) -> PhysicalPackage {
        loop {
            let return_package = self.input_buffer.pop_by_ref(
                2 * self.frame_length * self.input_descriptor.sample_rate as usize
                    / self.speed as usize,
                |data| {
                    // let current = std::time::Instant::now();
                    frame::frame_resolve_to_bitvec(
                        data,
                        &self.header,
                        &self.multiplex_frequency,
                        self.input_descriptor.sample_rate,
                        self.speed,
                        self.frame_length,
                    )
                },
            );
            if let Some(package) = return_package {
                return PhysicalPackage{
                    0:package
                }
            }
        }
    }

    fn receive_time_out(&mut self) -> Option<PhysicalPackage> {
        let mut count = 0;
        let gateway = 2 * self.frame_length * self.input_descriptor.sample_rate as usize
            / self.speed as usize + HEADER_LENGTH * 2 + 10;
        loop {
            std::thread::sleep(std::time::Duration::from_millis(3));
            if self.input_buffer.len() < gateway {
                count += 1;
                if count > TIME_OUT { return None; }
                else { continue; }
            }
            let return_package = self.input_buffer.pop_by_ref(
                gateway,
                |data| {
                    // let current = std::time::Instant::now();
                    frame::frame_resolve_to_bitvec(
                        data,
                        &self.header,
                        &self.multiplex_frequency,
                        self.input_descriptor.sample_rate,
                        self.speed,
                        self.frame_length,
                    )
                },
            );
            if let Some(package) = return_package {
                return Some(PhysicalPackage{
                    0:package
                })
            } else {
                count += 1;
                if count > TIME_OUT { return None; }
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
        use rand::prelude::*;
        use rand_pcg::Pcg64;

        let mut rng = Pcg64::seed_from_u64(2);
        let vec: Vec<u8> = (0..size).map(|_| rng.gen()).collect();
        let data = BitVec::from_vec(vec);
        let mut samples = frame::generate_frame_sample_from_bitvec(
            &data,
            header,
            multiplex_frequency,
            48000,
            SPEED,
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
            12000,
            &mut (0..12000)
                .map(|x| (x as f32 * 6.28 * 3000.0 / 48000.0).sin() * 0.5)
                .take(12000),
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
            frame::frame_resolve_to_bitvec(data, &header, &[5000.0], 48000, SPEED, 512)
        });
        let result = result.unwrap();
        println!("result:\t{:?}", result);
        println!("source:\t{:?}", ground_truth);
        assert_eq!(result, ground_truth)
    }

    #[test]
    fn test_decode_frame_from_physical_layer() {
        const SIZE: usize = 50;
        const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0];
        // const FREQUENCY: &'static [f32] = &[4000.0, 5000.0];
        const FRAME_SIZE: usize = 30;
        let mut layer = PhysicalLayer::new(FREQUENCY, SIZE);
        let header = layer.header.clone();
        let output_buffer = layer.output_buffer.clone();
        let handle = std::thread::spawn(move || {
            push_data_to_buffer(&*output_buffer, SIZE, FRAME_SIZE, &header, FREQUENCY)
        });
        let ground_truth = handle.join().unwrap();
        println!("{}", ground_truth);
        // cs140_util::record::record();
        let mut response: super::BitStore = BitVec::new();
        for _ in 0..FRAME_SIZE {
            response.extend(layer.receive().0.iter());
        }
        println!("result:\t{:?}", response);
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
        const SIZE: usize = 50;
        const FREQUENCY: &'static [f32] = &[1000.0, 2000.0, 3000.0, 4000.0, 5000.0, 6000.0, 7000.0, 8000.0];
        const FRAME_SIZE: usize = 30;
        let header = create_header(HEADER_LENGTH, MIN_FREQUENCY, MAX_FREQUENCY, 48000);
        let mut tmp: Vec<_> = (0..30000)
            .map(|x| (x as f32 * 6.28 * 3000.0 / 48000.0).sin() * 0.5)
            .take(30000)
            .collect();
        for _ in 0..FRAME_SIZE {
            let (samples, _) = generate_data(SIZE, &header, FREQUENCY);
            tmp.extend(samples.iter());
        }
        let tmp_: Vec<_> = std::iter::repeat(0.0).take(10000).collect();
        tmp.extend(tmp_.iter());
        // cs140_util::record::record_from_slice("/Users/vixbob/cs140/output.wav",tmp.as_slice());
        let thread = std::thread::spawn(|| cs140_util::record::record("/Users/vixbob/cs140/output.wav", 10));
        let buffer: RingBuffer<f32, 100000, false> = RingBuffer::new();
        let buffer_ptr = Arc::new(buffer);
        let (output, descriptor) = OutputDevice::new(buffer_ptr.clone());
        let close_output = output.play();
        for samples in tmp.chunks(100) {
            buffer_ptr.push_by_ref(samples);
        }
        std::thread::sleep(std::time::Duration::from_secs(3));
        close_output();
    }
}
