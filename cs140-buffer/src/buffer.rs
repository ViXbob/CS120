use crate::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;

impl<T, const N: usize, const GARBAGE_COLLECTION: bool> Buffer<T>
    for RingBuffer<T, N, GARBAGE_COLLECTION>
where
    T: Sync + Send,
{
    fn push(&self, count: usize, producer: impl FnOnce(&mut [T], &mut [T]) -> usize) {
        self.push(count, producer);
    }

    fn try_push(
        &self,
        count: usize,
        producer: impl FnOnce(&mut [T], &mut [T]) -> usize,
    ) -> Option<()> {
        self.try_push(count, producer)
    }

    fn pop<U>(&self, count: usize, consumer: impl FnOnce(&[T], &[T]) -> (U, usize)) -> U {
        self.pop(count, consumer)
    }

    fn try_pop<U>(
        &self,
        count: usize,
        consumer: impl FnOnce(&[T], &[T]) -> (U, usize),
    ) -> Option<U> {
        self.try_pop(count, consumer)
    }
}
