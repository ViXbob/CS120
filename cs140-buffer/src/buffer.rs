use crate::ring_buffer::RingBuffer;
use async_trait::async_trait;
use cs140_common::buffer::Buffer;

#[async_trait]
impl<T, const N: usize> Buffer<T> for RingBuffer<T, N>
    where
        T: Sync + Send + Copy,
{
    async fn push(
        &self,
        count: usize,
        producer: impl for<'a> FnOnce(&'a mut [T], &'a mut [T]) -> usize + Send + 'async_trait,
    ) {
        self.push(count, producer).await
    }

    async fn pop<U>(
        &self,
        count: usize,
        consumer: impl for<'a> FnOnce(&'a [T], &'a [T]) -> (U, usize) + Send + 'async_trait,
    ) -> U {
        self.pop(count, consumer).await
    }
    fn must_pop<U>(
        &self,
        count: usize,
        consumer: impl FnOnce(&[T], &[T]) -> (U, usize),
        producer: impl Iterator<Item = T>,
    ) -> U {
        self.must_pop(count, consumer, producer)
    }
}
