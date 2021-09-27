use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::device::{InputDevice, OutputDevice};
use std::sync::Arc;

fn main() {
    let buffer: RingBuffer<f32, 100000, false> = RingBuffer::new();
    let buffer_ptr = Arc::new(buffer);
    let input = InputDevice::new(buffer_ptr.clone()).0;
    let output = OutputDevice::new(buffer_ptr.clone()).0;
    let close_input = input.listen();
    let close_play = output.play();
    std::thread::sleep(std::time::Duration::from_millis(100000));
}
