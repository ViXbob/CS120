// use cs140_buffer::{ring_buffer::RingBuffer};
// use cs140_common::{device::InputDevice, device::OutputDevice};
// use std::sync::{Arc, Mutex};
//
// pub fn task1_ck1() {
//     let buffer: RingBuffer<f32, 500000, false> = RingBuffer::new();
//     let buffer_ptr = Arc::new(buffer);
//     let input = InputDevice::new(buffer_ptr.clone());
//     // let output = OutputDevice::new(buffer_ptr.clone());
//     // let input = std::thread::spawn(move || input.listen());
//     // let output = std::thread::spawn(move || output.play());
//     // input.join();
//     // output.join();
//
//     // writer.lock().unwrap().take().unwrap().finalize()?;
//     // println!("Recording {} complete!", PATH);
//
//     fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
//         match format {
//             cpal::SampleFormat::U16 => hound::SampleFormat::Int,
//             cpal::SampleFormat::I16 => hound::SampleFormat::Int,
//             cpal::SampleFormat::F32 => hound::SampleFormat::Float,
//         }
//     }
//
//     fn wav_spec_from_config(input: &InputDevice<RingBuffer<f32, 500000, false>>) -> hound::WavSpec {
//         hound::WavSpec {
//             channels: input.stream_config.1.channels as _,
//             sample_rate: input.stream_config.1.sample_rate.0 as _,
//             bits_per_sample: (input.stream_config.2.sample_size() * 8) as _,
//             sample_format: sample_format(input.stream_config.2),
//         }
//     }
//
//     const PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/recorded.wav");
//     let spec = wav_spec_from_config(&input);
//     let writer = hound::WavWriter::create(PATH, spec)?;
//     let writer = Arc::new(Mutex::new(Some(writer)));
//
//     // A flag to indicate that recording is in progress.
//     println!("Begin recording...");
//
//
//     input.listen();
//     std::thread::sleep(std::time::Duration::from_secs(10));
//     if let Ok(mut guard) = &writer.try_lock() {
//         if let Some(writer) = guard.as_mut() {
//
//             for &sample in input.iter() {
//                 let sample: impl cpal::Sample + hound::Sample = cpal::Sample::from(&sample);
//                 writer.write_sample(sample).ok();
//             }
//         }
//     }
//     drop(input);
//     writer.lock().unwrap().take().unwrap().finalize()?;
//     println!("Recording {} complete!", PATH);
//     println!("1234");
// }