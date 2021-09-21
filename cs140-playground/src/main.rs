use cs140_common::device::{InputDevice, OutputDevice};
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_buffer::audio_data::AudioData;
use std::sync::Arc;

fn main() {
    let buffer:RingBuffer<AudioData<512>,100> = RingBuffer::new();
    let buffer_ptr = Arc::new(buffer);
    let input = InputDevice::new(buffer_ptr.clone());
    let output = OutputDevice::new(buffer_ptr.clone());
    let input = std::thread::spawn(move ||input.listen());
    let output = std::thread::spawn(move||output.play());
    input.join();
    output.join();
}