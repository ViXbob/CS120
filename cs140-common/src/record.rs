// use crate::buffer::Buffer;
// use hound::WavWriter;
// use std::sync::Arc;
//
// pub struct Recorder<Writer>
// where
//     Writer: std::io::Write + std::io::Seek,
// {
//     writer: WavWriter<Writer>,
//     data_count: usize,
//     data_written: usize,
// }
//
// impl<Writer> Recorder<Writer>
// where
//     Writer: std::io::Write + std::io::Seek,
// {
//     pub fn new(writer: WavWriter<Writer>, data_count: usize) -> Self {
//         Recorder {
//             writer,
//             data_count,
//             data_written: 0,
//         }
//     }
//
//     pub fn record_from_buffer<T: Buffer<f32>>(mut self, buffer: Arc<T>, segment_len: usize) {
//         loop {
//             let result = buffer.pop_by_ref(segment_len as usize, |data| {
//                 (self.record_from_slice(data), data.len())
//             });
//             match result {
//                 None => {
//                     return;
//                 }
//                 Some(val) => self = val,
//             }
//         }
//     }
//
//     pub fn record_from_slice(mut self, mut data: &[f32]) -> Option<Self> {
//         if self.data_written + data.len() >= self.data_count {
//             data = &data[..(self.data_count - self.data_written)];
//             self.data_written = self.data_count;
//             write_input_data::<f32, f32, _>(data, &mut self.writer);
//             self.writer.finalize().unwrap();
//             None
//         } else {
//             self.data_written += data.len();
//             write_input_data::<f32, f32, _>(data, &mut self.writer);
//             Some(self)
//         }
//     }
// }
//
// fn write_input_data<T, U, Writer>(input: &[T], writer: &mut WavWriter<Writer>)
// where
//     T: cpal::Sample,
//     U: cpal::Sample + hound::Sample,
//     Writer: std::io::Write + std::io::Seek,
// {
//     for &sample in input.iter() {
//         let sample: U = cpal::Sample::from(&sample);
//         for _i in 0..writer.spec().channels {
//             writer.write_sample(sample).ok();
//         }
//     }
// }
