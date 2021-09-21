use crate::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;

impl<T, const N: usize, const GarbageCollection: bool> Buffer<T>
    for RingBuffer<T, N, GarbageCollection>
where
    T: Sync + Send,
{
    fn push(&self, count: usize, producer: impl FnMut(&mut [T], &mut [T])) {
        self.push(count, producer);
    }

    fn pop<U>(&self, count: usize, consumer: impl FnMut(&[T], &[T]) -> U) -> U {
        self.pop(count, consumer)
    }
}
