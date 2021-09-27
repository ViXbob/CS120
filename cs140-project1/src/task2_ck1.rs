use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::device::{InputDevice, OutputDevice};
use std::sync::Arc;
use cs140_common::buffer::Buffer;

fn main() {
    let buffer: RingBuffer<f32, 100000, false> = RingBuffer::new();
    let buffer_ptr = Arc::new(buffer);
    let (output, config) = OutputDevice::new(buffer_ptr.clone());
    // println!("{}", config.0.channels);
    let buffer_push_ptr = buffer_ptr.clone();
    let record_time = 5;
    std::thread::spawn(move || {
        let segment_count = 100;
        let segment_len = config.0.sample_rate.0 / segment_count;
        for i in 0..record_time*segment_count {
            let segment_index = i % segment_count;
            let rate = config.0.sample_rate.0 as f32;
            buffer_push_ptr.push_by_iterator(segment_len as usize,
                                             (segment_index*segment_len..(segment_index+1)*segment_len).map(
                                                 |x : _| {
                                                     let xy = x as f32 * 2.0 * std::f32::consts::PI / rate;
                                                     (xy * 1000_f32).sin() + (xy * 10000_f32).sin()
                                                 }
                                             ).by_ref()
            );
        }
    });
    let close_play = output.play();
    std::thread::sleep(std::time::Duration::from_secs(record_time as u64));
    close_play();
}