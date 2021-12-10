use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};

const DEFAULT_PUSH_BLOCKING_SIZE: usize = 48000;

pub struct BlockingRingBuffer<T, const N: usize> {
    buffer: Box<[T; N]>,
    head: usize,
    len: usize,
    push_waker: VecDeque<Waker>,
    pop_waker: VecDeque<Waker>,
    push_blocking_size: usize,
}

pub struct RingBuffer<T, const N: usize>(Mutex<BlockingRingBuffer<T, N>>);

impl<T, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            0: Mutex::new(BlockingRingBuffer::new()),
        }
    }

    pub fn must_pop<U>(
        &self,
        count: usize,
        consumer: impl FnOnce(&[T], &[T]) -> (U, usize),
        producer: impl Iterator<Item=T>,
    ) -> U where T: Copy {
        let mut guard = self.0.lock().unwrap();
        let len = guard.len();
        let result = if count <= len {
            guard.pop_blocking(count, consumer)
        } else {
            guard.pop_blocking(len, |first: &[T], second: &[T]| {
                let data: Vec<_> = first.iter().cloned().chain(second.iter().cloned()).collect();
                let padding: Vec<_> = producer.take(count - len).collect();
                let (value, _) = consumer(&data, &padding);
                (value, len)
            })
        };
        guard.push_blocking_size = if guard.push_blocking_size == DEFAULT_PUSH_BLOCKING_SIZE && count > 0{
            count * 2
        } else {
            std::cmp::max(guard.push_blocking_size, count * 2)
        };
        for push_waker in &guard.push_waker {
            push_waker.wake_by_ref();
        }
        guard.push_waker.clear();
        result
    }


    pub async fn push(
        &self,
        count: usize,
        producer: impl for<'a> FnOnce(&'a mut [T], &'a mut [T]) -> usize,
    ) {
        RingBufferPushFuture {
            buffer: &self.0,
            push_len_required: count,
            push_fn: producer,
        }
            .await
    }

    pub async fn pop<U>(
        &self,
        count: usize,
        consumer: impl for<'a> FnOnce(&'a [T], &'a [T]) -> (U, usize),
    ) -> U {
        RingBufferPopFuture {
            buffer: &self.0,
            pop_len_required: count,
            pop_fn: consumer,
        }
            .await
    }

    pub fn len(&self) -> usize {
        self.0.lock().unwrap().len
    }
}

pub struct RingBufferPushFuture<'a, PushCallback, T, const N: usize>
    where
        PushCallback: for<'b> FnOnce(&'b mut [T], &'b mut [T]) -> usize,
{
    buffer: &'a Mutex<BlockingRingBuffer<T, N>>,
    push_len_required: usize,
    push_fn: PushCallback,
}

pub struct RingBufferPopFuture<'a, U, PopCallback, T, const N: usize>
    where
        PopCallback: for<'b> FnOnce(&'b [T], &'b [T]) -> (U, usize),
{
    buffer: &'a Mutex<BlockingRingBuffer<T, N>>,
    pop_len_required: usize,
    pop_fn: PopCallback,
}

impl<'a, PushCallback, T, const N: usize> Future for RingBufferPushFuture<'a, PushCallback, T, N>
    where
        PushCallback: for<'b> FnOnce(&'b mut [T], &'b mut [T]) -> usize,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.buffer.lock().unwrap();
        if guard.capacity() - guard.len() >= self.push_len_required && guard.len() < guard.push_blocking_size {
            let push_fn: PushCallback = unsafe { std::mem::transmute_copy(&self.push_fn) };
            guard.push_blocking(push_fn);
            for pop_waker in &guard.pop_waker {
                pop_waker.wake_by_ref();
            }
            guard.pop_waker.clear();
            // log::warn!("push head: {}",guard.head);
            Poll::Ready(())
        } else {
            guard.push_waker.push_back(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<'a, U, PopCallback, T, const N: usize> Future for RingBufferPopFuture<'a, U, PopCallback, T, N>
    where
        PopCallback: for<'b> FnOnce(&'b [T], &'b [T]) -> (U, usize),
{
    type Output = U;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut guard = self.buffer.lock().unwrap();
        if guard.len() >= self.pop_len_required {
            let pop_fn: PopCallback = unsafe { std::mem::transmute_copy(&self.pop_fn) };
            let result = guard.pop_blocking(self.pop_len_required, pop_fn);
            for push_waker in &guard.push_waker {
                push_waker.wake_by_ref();
            }
            guard.push_waker.clear();
            // log::warn!("pop head: {}",guard.head);
            Poll::Ready(result)
        } else {
            guard.pop_waker.push_back(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<T, const N: usize> BlockingRingBuffer<T, N> {
    #[allow(clippy::uninit_assumed_init)]
    pub fn new() -> Self {
        BlockingRingBuffer {
            buffer: unsafe { Box::new_uninit().assume_init() },
            head: 0,
            len: 0,
            push_waker: Default::default(),
            pop_waker: Default::default(),
            push_blocking_size: DEFAULT_PUSH_BLOCKING_SIZE,
        }
    }

    fn len(&self) -> usize {
        self.len
    }

    fn capacity(&self) -> usize {
        N
    }

    fn push_blocking(&mut self, producer: impl for<'a> FnOnce(&'a mut [T], &'a mut [T]) -> usize) {
        let head = self.head;
        let tail = (head + self.len()) % N;

        let count = unsafe {
            if head <= tail {
                let first_ptr = self.buffer[tail..].as_ptr();
                let first_ptr = first_ptr as *mut T;
                let second_ptr = self.buffer[..head].as_ptr();
                let second_ptr = second_ptr as *mut T;
                producer(
                    std::slice::from_raw_parts_mut(first_ptr, N - tail),
                    std::slice::from_raw_parts_mut(second_ptr, head),
                )
            } else {
                let first = &mut self.buffer[tail..head];
                producer(first, &mut [])
            }
        };

        self.len += count;
    }

    fn pop_blocking<U>(
        &mut self,
        count: usize,
        consumer: impl for<'a> FnOnce(&'a [T], &'a [T]) -> (U, usize),
    ) -> U {
        let head = self.head;
        let tail = (head + self.len()) % N;
        let (result, count) = {
            if head < tail {
                let slice = &self.buffer[head..tail];
                let (value, count) = consumer(slice, &[]);
                (value, count)
            } else {
                let first_slice = &self.buffer[head..];
                if first_slice.len() >= count {
                    let first_slice = &first_slice[..count];
                    let (value, count) = consumer(first_slice, &[]);
                    (value, count)
                } else {
                    let second_slice = &self.buffer[..count - first_slice.len()];
                    let (value, count) = consumer(first_slice, second_slice);
                    (value, count)
                }
            }
        };
        self.head = (head + count) % N;
        self.len -= count;
        result
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn test_timeout() {
        let buffer = Arc::new(RingBuffer::<i32, 1000000>::new());
        buffer.push(4, |x, y| {
            x[0] = 1;
            x[1] = 2;
            x[2] = 3;
            x[3] = 4;
            4
        }).await;
        assert_eq!(buffer.pop(1, |x, y| (x[0], 1)).await, 1);
        assert_eq!(buffer.pop(1, |x, y| (x[0], 1)).await, 2);
        assert_eq!(buffer.pop(1, |x, y| (x[0], 1)).await, 3);
        assert_eq!(buffer.pop(1, |x, y| (x[0], 1)).await, 4);
    }
}
