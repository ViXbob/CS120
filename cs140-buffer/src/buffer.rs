use crate::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;

impl<T, const N: usize, const GARBAGE_COLLECTION: bool> Buffer<T>
    for RingBuffer<T, N, GARBAGE_COLLECTION>
where
    T: Sync + Send,
{
    fn push(&self, count: usize, producer: impl FnOnce(&mut [T], &mut [T])) {
        self.push(count, producer);
    }

    fn pop<U>(&self, count: usize, consumer: impl FnOnce(&[T], &[T]) -> U) -> U {
        self.pop(count, consumer)
    }
}
