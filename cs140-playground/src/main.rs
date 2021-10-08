use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::device::OutputDevice;
use cs140_frame_handler::header::generate_frame_sample;
use rand::Rng;
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::Arc;

fn play_audio() {
    let buffer: RingBuffer<f32, 100000, false> = RingBuffer::new();
    let buffer_ptr = Arc::new(buffer);
    let (output, config) = OutputDevice::new(buffer_ptr.clone());
    let record_time: f32 = 10.0;
    std::thread::spawn(move || {
        let sample_rate = config.sample_rate;
        let header_length = 12000;
        let segment_count = 200;
        let segment_len = header_length / segment_count;
        let scale: f32 = 1.0;
        let min_frequency: f32 = 200.0;
        let max_frequency: f32 = 20000.0;

        let mut phase: f32 = 0.0;
        let mut cur_frequency: f32 = min_frequency;
        let frequency_step = (max_frequency - min_frequency) / (header_length as f32 / 2.0);
        let time_gap: f32 = 1.0 / sample_rate as f32;

        // println!("{}", frequency_step);

        for i in 0.. {
            let segment_index = i % segment_count;
            buffer_ptr.push_by_iterator(
                segment_len as usize,
                (segment_index * segment_len..(segment_index + 1) * segment_len)
                    .map(|x: _| {
                        println!("{}", x);
                        if 2 * x < header_length {
                            cur_frequency += frequency_step;
                            // println!("111");
                        } else {
                            cur_frequency -= frequency_step;
                            // println!("000");
                        }
                        println!("{}", cur_frequency);
                        phase += 2.0 * std::f32::consts::PI * time_gap * cur_frequency;
                        phase.sin() * scale
                    })
                    .by_ref(),
            );
        }
    });
    let close_play = output.play();
    std::thread::sleep(std::time::Duration::from_secs(record_time as u64));
    close_play();
}

fn play_audio_from_vector(data: Vec<f32>, record_time: f32) {
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
    cs140_util::record::record(PATH1, 10);
    std::thread::sleep(std::time::Duration::from_secs(record_time as u64));
    close_play();
}

fn fft_test() {
    let mut planner = FftPlanner::new();
    let fft_len = 192;
    let fft = planner.plan_fft_forward(fft_len);
    // let mut buffer = vec![Complex{ re: 0.0f32, im: 0.0f32 }; 4096];
    let fre1: f32 = 1000.0 * 2.0 * std::f32::consts::PI;
    let fre2: f32 = 2000.0 * 2.0 * std::f32::consts::PI;
    let fre3: f32 = 3000.0 * 2.0 * std::f32::consts::PI;
    // 48, 24, 16
    //
    let mut buffer: Vec<_> = (0..fft_len)
        .map(|x| {
            let x: f32 = x as f32 / 48000.0f32;
            Complex {
                re: (x * fre1).sin() + (x * fre2).cos() + (x * fre3).sin(),
                im: 0.0f32,
            }
        })
        .collect();
    fft.process(&mut buffer);
    for (index, x) in buffer.iter().enumerate() {
        println!("{} : {:?}", index, x);
    }
    // println!("{:?}, {:?}, {:?}", buffer.get(333), buffer.get(450), buffer.get(888));
}

fn main() {
    let record_time = 10;
    let data = (0..(record_time * 1000))
        .map(|_| rand::thread_rng().gen_range(0..2))
        .collect::<Vec<i32>>();
    let multiplex_frequency: [f32; 1] = [10000.0];
    let data = generate_frame_sample(
        data.as_slice(),
        multiplex_frequency.len(),
        &multiplex_frequency,
        48000,
        1000,
    );
    // println!("{:?}", data);
    play_audio_from_vector(data, record_time as f32);
}
