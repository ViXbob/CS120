use crate::ring_buffer::RingBuffer;
use cs140_common::data_buffer::DataBuffer;

impl<T: Send + Sync, const N: usize> DataBuffer for RingBuffer<T, N> {
    type Data = T;

    fn push(&self, data: Self::Data) {
        self.push(data);
    }

    fn pop<U>(&self, consumer: impl FnMut(&Self::Data) -> U) -> U {
        self.pop(consumer)
    }
}
