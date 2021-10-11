use super::header;
use crate::encoding::BitStore;
use bitvec::order::Lsb0;
use bitvec::vec::BitVec;
use rustfft::{num_complex::Complex, FftPlanner};

pub fn frame_resolve(
    data: &[f32],
    header_length: usize,
    header: &[f32],
    multiplex_range: usize,
    multiplex_frequency: &[f32],
    sample_rate: u32,
    speed: u32,
    frame_length: usize,
) -> Result<(Vec<i32>, usize), &'static str> {
    let begin_index = header::header_detect(data, header_length, header).expect("detection failed");
    let sample_per_bit = sample_rate / speed;
    let fft_len: usize = sample_per_bit as usize;
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_len);
    // let mut buffer: Vec<Complex<f32>> = Vec::new();
    let mut result: Vec<i32> = Vec::new();
    for i in 0..frame_length {
        let buffer = data[(begin_index + i * sample_per_bit as usize)
            ..(begin_index + (i + 1) * sample_per_bit as usize)]
            .iter()
            .map(|x: _| Complex::<f32>::new(*x, 0.0));
        // let mut buffer : Vec<_> = buffer.skip(sample_per_bit as usize / 4).take(sample_per_bit as usize / 2).cycle().take(sample_per_bit as usize).collect();
        let mut buffer: Vec<_> = buffer.collect();
        fft.process(buffer.as_mut_slice());
        for frequency in multiplex_frequency {
            let index: usize = (*frequency as usize) / ((sample_rate / sample_per_bit) as usize);
            let value = buffer[index];
            println!("{}", value.im / (sample_per_bit as f32) * 2.0);
            if (value.im.abs() / (sample_per_bit as f32) * 2.0 > 0.01) && (value.im < 0.0) {
                result.push(1);
            } else {
                result.push(0);
            }
            // if (value.im.abs() / (sample_per_bit as f32) * 2.0 > 0.01) && (value.im > 0.0)
        }
    }
    Ok((
        result,
        (begin_index + frame_length * sample_per_bit as usize) as usize,
    ))
}

pub fn frame_resolve_to_bitvec(
    data: &[f32],
    header_length: usize,
    header: &[f32],
    multiplex_frequency: &[f32],
    sample_rate: u32,
    speed: u32,
    frame_length: usize,
) -> (Option<BitStore>, usize) {
    let begin_index = header::header_detect(data, header_length, header); //.expect("detection failed");
    if begin_index.is_none() {
        return (None, data.len() - header.len());
    }
    let begin_index = begin_index.unwrap();
    let sample_per_bit = sample_rate / speed;
    let fft_len: usize = sample_per_bit as usize;
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_len);
    // let mut buffer: Vec<Complex<f32>> = Vec::new();
    let mut result: BitStore = BitVec::new();
    for i in 0..frame_length {
        let buffer = data[(begin_index + i * sample_per_bit as usize)
            ..(begin_index + (i + 1) * sample_per_bit as usize)]
            .iter()
            .map(|x: _| Complex::<f32>::new(*x, 0.0));
        // let mut buffer : Vec<_> = buffer.skip(sample_per_bit as usize / 4).take(sample_per_bit as usize / 2).cycle().take(sample_per_bit as usize).collect();
        let mut buffer: Vec<_> = buffer.collect();
        fft.process(buffer.as_mut_slice());
        for frequency in multiplex_frequency {
            let index: usize = (*frequency as usize) / ((sample_rate / sample_per_bit) as usize);
            let value = buffer[index];
            println!("{}", value.im / (sample_per_bit as f32) * 2.0);
            if (value.im.abs() / (sample_per_bit as f32) * 2.0 > 0.01) && (value.im < 0.0) {
                result.push(true);
            } else {
                result.push(false);
            }
            // if (value.im.abs() / (sample_per_bit as f32) * 2.0 > 0.01) && (value.im > 0.0)
        }
    }
    (
        Some(result),
        (begin_index + frame_length * sample_per_bit as usize) as usize,
    )
}

pub fn generate_frame_sample(
    data: &[i32],
    multiplex_range: usize,
    multiplex_frequency: &[f32],
    sample_rate: u32,
    speed: u32,
) -> Vec<f32> {
    assert!(multiplex_range > 0);
    let samples_per_bit: f32 = (sample_rate / speed) as f32;
    let mut rtn: Vec<f32> = header::header_create(220, 3000.0, 6000.0, sample_rate, 1.0);
    let sample_rate: f32 = sample_rate as f32;
    for (i, bits_group) in data.chunks(multiplex_range).enumerate() {
        for time in i * (samples_per_bit as usize)..(i + 1) * (samples_per_bit as usize) {
            let phase: f32 = 2.0 * std::f32::consts::PI * time as f32 / sample_rate;
            let mut value: f32 = 0.0;
            for (j, &bit) in bits_group.iter().enumerate() {
                value += if bit == 1 {
                    (phase * multiplex_frequency[j]).sin() * 1.0
                } else {
                    -(phase * multiplex_frequency[j]).sin() * 1.0
                }
            }
            rtn.push(value);
        }
    }
    rtn
}

pub fn generate_frame_sample_from_bitvec(
    data: &BitStore,
    multiplex_range: usize,
    multiplex_frequency: &[f32],
    sample_rate: u32,
    speed: u32,
) -> Vec<f32> {
    assert!(multiplex_range > 0);
    let samples_per_bit: f32 = (sample_rate / speed) as f32;
    let mut rtn: Vec<f32> = header::header_create(220, 3000.0, 6000.0, sample_rate, 1.0);
    let sample_rate: f32 = sample_rate as f32;
    for (i, bits_group) in data.chunks(multiplex_range).enumerate() {
        for time in i * (samples_per_bit as usize)..(i + 1) * (samples_per_bit as usize) {
            let phase: f32 = 2.0 * std::f32::consts::PI * time as f32 / sample_rate;
            let mut value: f32 = 0.0;
            for (j, bit) in bits_group.iter().enumerate() {
                value += if *bit {
                    (phase * multiplex_frequency[j]).sin() * 1.0
                } else {
                    -(phase * multiplex_frequency[j]).sin() * 1.0
                }
            }
            rtn.push(value);
        }
    }
    rtn
}

#[cfg(test)]
mod test {
    use std::sync::Arc;
    use cs140_buffer::ring_buffer::RingBuffer;
    use cs140_common::buffer::Buffer;
    use cs140_common::descriptor::{SampleFormat, SoundDescriptor};
    use cs140_common::device::OutputDevice;
    use cs140_common::record::Recorder;
    use crate::framing::frame::generate_frame_sample;
    use crate::framing::header;

    #[test]
    fn calculate_power_of_header() {
        let data = header::header_create(440, 2000.0, 10000.0, 48000, 1.0);
        println!("{}", data.iter().map(|x: _| { x * x }).sum::<f32>());
        println!("{}", data.len());
    }

    #[test]
    fn header_detect_test() -> Result<(), anyhow::Error> {
        const PATH: &str = "/Users/vixbob/cs140/cs140-playground/recorded1.wav";
        let data = read_from_file_to_vec(PATH);
        let header = cs140_frame_handler::header::header_create(220, 3000.0, 6000.0, 48000, 1.0);
        let first_index = cs140_frame_handler::header::header_detect(&data, 220, &header)
            .expect("detection failed");
        println!("{}", first_index);
        Ok(())
    }

    #[test]
    fn generate_data_with_noise() {
        let record_time = 1;
        let data = (0..(record_time * 1000))
            .map(|_| rand::thread_rng().gen_range(0..2))
            .collect::<Vec<i32>>();
        let multiplex_frequency: [f32; 1] = [10000.0];
        // 2 * 2 * 2 * 2 * 3
        // 4, 8, 16, 6, 12, 24
        // 12000, 6000, 3000, 8000, 4000, 2000
        // 2000, 3000, 4000, 6000, 8000, 12000
        // 2000, 3000, 4000, 6000, 8000, 12000
        // 12K, 15K, 16K, 18K, 16K, 12K
        let data = generate_frame_sample(
            data.as_slice(),
            multiplex_frequency.len(),
            &multiplex_frequency,
            48000,
            1000,
        );
        let data = data
            .iter()
            .map(|x: _| {
                x + rand::thread_rng()
                    .gen_range(-std::f32::consts::PI..std::f32::consts::PI)
                    .cos()
                    * 0.4
            })
            .collect::<Vec<f32>>();
        let data = (0..5000)
            .map(|_| {
                rand::thread_rng()
                    .gen_range(-std::f32::consts::PI..std::f32::consts::PI)
                    .sin()
                    * 0.1
            })
            .chain(data.iter().cloned())
            .collect::<Vec<f32>>();
        let descriptor = SoundDescriptor {
            channels: 1,
            sample_rate: 48000,
            sample_format: SampleFormat::F32,
        };
        let writer = WavWriter::create(
            concat!(env!("CARGO_MANIFEST_DIR"), "/output.wav"),
            descriptor.clone().into(),
        )
        .unwrap();
        let recorder = Recorder::new(writer, data.len() as usize);
        recorder.record_from_slice(&data);
    }

    #[test]
    fn generate_noise() {
        let data = (0..12000)
            .map(|_| {
                rand::thread_rng()
                    .gen_range(-std::f32::consts::PI..std::f32::consts::PI)
                    .sin()
                    * 1.0
            })
            .collect::<Vec<f32>>();
        let descriptor = SoundDescriptor {
            channels: 1,
            sample_rate: 48000,
            sample_format: SampleFormat::F32,
        };
        let writer = WavWriter::create(
            concat!(env!("CARGO_MANIFEST_DIR"), "/noise.wav"),
            descriptor.clone().into(),
        )
            .unwrap();
        let recorder = Recorder::new(writer, data.len() as usize);
        recorder.record_from_slice(&data.as_slice());
    }

    #[test]
    fn frame_resolve_test() {
        const PATH: &str = "/Users/vixbob/cs140/cs140-playground/recorded1.wav";
        let data = read_from_file_to_vec(PATH);
        // println!("{:?}", data);
        let multiplex_frequency: [f32; 1] = [5000.0];
        let header = cs140_frame_handler::header::header_create(220, 3000.0, 6000.0, 48000, 1.0);
        let (result, next_index) = cs140_frame_handler::frame::frame_resolve(
            data.as_slice(),
            220,
            header.as_slice(),
            1,
            &multiplex_frequency,
            48000,
            1000,
            1000,
        )
        .unwrap();
        println!("{:?}", result);
        println!("{}", next_index);
    }

    #[test]
    fn play_audio_from_vector_and_record() {
        let record_time = 1;
        let data = (0..(record_time * 1000))
            .map(|_| rand::thread_rng().gen_range(0..2))
            .collect::<Vec<i32>>();
        // let data = (0..(record_time * 1000))
        //     .map(|x : _| x % 2)
        //     .collect::<Vec<i32>>();
        println!("{:?}", data);
        let multiplex_frequency: [f32; 1] = [5000.0];
        let data = vec![
            read_from_file_to_vec("/Users/vixbob/cs140/cs140-playground/noise.wav"),
            generate_frame_sample(
            data.as_slice(),
            multiplex_frequency.len(),
            &multiplex_frequency,
            48000,
            1000,
        )].concat();
        let buffer: RingBuffer<f32, 100000, false> = RingBuffer::new();
        let buffer_ptr = Arc::new(buffer);
        let (output, _) = OutputDevice::new(buffer_ptr.clone());
        let segment_length = 100;
        std::thread::spawn(move || {
            for segment in data.chunks(segment_length) {
                buffer_ptr.push_by_ref(segment);
            }
        });
        let close_play = output.play();
        const PATH1: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded1.wav");
        cs140_util::record::record(PATH1, (record_time + 1) as usize);
        std::thread::sleep(std::time::Duration::from_secs((record_time + 1) as u64));
        close_play();
    }
}
