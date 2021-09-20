use crate::ring_buffer::RingBuffer;
use cs140_common::audio_buffer::{AudioBuffer, AudioData};

impl<T: Send + Sync + AudioData, const N: usize> AudioBuffer for RingBuffer<T, N> {
    type Data = T;

    fn push(&self, data: Self::Data) {
        self.push(data);
    }

    fn pop<U>(&self, consumer: impl FnMut(&Self::Data) -> U) -> U {
        self.pop(consumer)
    }
}
