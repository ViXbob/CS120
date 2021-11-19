// use cs140_buffer::ring_buffer::RingBuffer;
// use cs140_common::descriptor::{SampleFormat, SoundDescriptor};
// use cs140_common::device::InputDevice;
// use cs140_common::record::Recorder;
// use hound::WavWriter;
// use std::sync::Arc;
//
// pub fn record(output_path: &str, record_time: usize) {
//     let buffer: RingBuffer<f32, 100000> = RingBuffer::new();
//     let buffer_ptr = Arc::new(buffer);
//     let (input, descriptor) = InputDevice::new(buffer_ptr.clone());
//     let close_input = input.listen();
//     let writer = WavWriter::create(output_path, descriptor.into()).unwrap();
//     let recorder = Recorder::new(writer, record_time * descriptor.sample_rate as usize);
//     let segment_len = 100;
//     recorder.record_from_buffer(buffer_ptr, segment_len);
//     close_input();
// }
//
// pub fn record_from_slice(output_path: &str, audio: &[f32]) {
//     let descriptor = SoundDescriptor {
//         channels: 1,
//         sample_rate: 48000,
//         sample_format: SampleFormat::F32,
//     };
//     let writer = WavWriter::create(output_path, descriptor.into()).unwrap();
//     let recorder = Recorder::new(writer, audio.len() as usize);
//     recorder.record_from_slice(audio);
// }
