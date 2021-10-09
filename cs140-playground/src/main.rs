use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::device::OutputDevice;
use cs140_frame_handler::header::generate_frame_sample;
use rand::Rng;
use hound::WavWriter;
use rustfft::{num_complex::Complex, FftPlanner};
use std::sync::Arc;
use cs140_common::descriptor::{SampleFormat, SoundDescriptor};
use cs140_common::record::Recorder;
use rodio::{source::Source, Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;


#[test]
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
                        if 2 * x < header_length {
                            cur_frequency += frequency_step;
                            // println!("111");
                        } else {
                            cur_frequency -= frequency_step;
                            // println!("000");
                        }
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


fn play_audio_from_vector_and_record() {
    let record_time = 1;
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

fn fft_test() {
    let mut planner = FftPlanner::new();
    let fft_len = 48;
    let fft = planner.plan_fft_forward(fft_len);
    // let mut buffer = vec![Complex{ re: 0.0f32, im: 0.0f32 }; 4096];
    // let fre1: f32 = 1000.0 * 2.0 * std::f32::consts::PI;
    // let fre2: f32 = 2000.0 * 2.0 * std::f32::consts::PI;
    let fre3: f32 = 3000.0 * 2.0 * std::f32::consts::PI;
    // 48, 24, 16
    //
    let mut buffer: Vec<_> = (0..fft_len)
        .map(|x| {
            let x: f32 = x as f32 / 48000.0f32;
            Complex {
                // re: (x * fre1).sin() + (x * fre2).cos() + (x * fre3).sin(),
                re: (x * fre3).sin(),
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

#[test]
fn calculate_power_of_header() {
    let data = cs140_frame_handler::header::header_create(440, 2000.0, 10000.0, 48000, 1.0);
    println!("{}", data.iter().map(|x : _| { x * x }).sum::<f32>());
    println!("{}", data.len());
}


fn header_detect_test() -> Result<(), anyhow::Error> {
    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/output.wav");
    println!("{}", PATH);
    // Get a output stream handle to the default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default()?;
    // Load a sound from a file, using a path relative to Cargo.toml
    let file = BufReader::new(File::open(PATH)?);
    // Decode that sound file into a source
    let source = Decoder::new(file)?;
    // Play the sound directly on the device
    // stream_handle.play_raw(source.convert_samples())?;
    let data = source.convert_samples().buffered().collect::<Vec<f32>>();
    // println!("{:?}", data);
    let header = cs140_frame_handler::header::header_create(220, 3000.0, 6000.0, 48000, 1.0);
    let first_index = cs140_frame_handler::header::header_detect(&data, 220, &header).expect("detection failed");
    println!("{}", first_index);
    Ok(())
}

fn frame_resolve_test() -> Result<(), anyhow::Error> {
    const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/output.wav");
    println!("{}", PATH);
    // Get a output stream handle to the default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default()?;
    // Load a sound from a file, using a path relative to Cargo.toml
    let file = BufReader::new(File::open(PATH)?);
    // Decode that sound file into a source
    let source = Decoder::new(file)?;
    // Play the sound directly on the device
    // stream_handle.play_raw(source.convert_samples())?;
    let data = source.convert_samples().buffered().collect::<Vec<f32>>();
    // println!("{:?}", data);
    let multiplex_frequency: [f32; 1] = [10000.0];
    let header = cs140_frame_handler::header::header_create(220, 3000.0, 6000.0, 48000, 1.0);
    let (result, next_index) = cs140_frame_handler::header::frame_resolve(data.as_slice(), 220, header.as_slice(), 1, &multiplex_frequency, 48000, 1000, 100).unwrap();
    println!("{:?}", result);
    println!("{}", next_index);
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
    let data = data.iter().map(|x : _| { x + rand::thread_rng().gen_range(-std::f32::consts::PI..std::f32::consts::PI).cos() * 0.4 }).collect::<Vec<f32>>();
    let data = (0..5000).map(|_| { rand::thread_rng().gen_range(-std::f32::consts::PI..std::f32::consts::PI).sin() * 0.1 }).chain(data.iter().cloned()).collect::<Vec<f32>>();
    let descriptor = SoundDescriptor{
        channels: 1,
        sample_rate: 48000,
        sample_format: SampleFormat::F32
    };
    let writer = WavWriter::create(concat!(env!("CARGO_MANIFEST_DIR"), "/output.wav"), descriptor.clone().into()).unwrap();
    let recorder = Recorder::new(writer, data.len() as usize);
    recorder.record_from_slice(&data);
}

fn main() {
    // play_audio_from_vector_and_record();
    // header_detect_test();
    // fft_test();
    // generate_data_with_noise();
    frame_resolve_test();
}
