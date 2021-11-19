use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;
use std::task::{Context, Poll, Waker};

pub struct BlockingRingBuffer<T, const N: usize> {
    buffer: Box<[T; N]>,
    head: usize,
    len: usize,
    push_waker: VecDeque<Waker>,
    pop_waker: VecDeque<Waker>,
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
        producer: impl Iterator<Item=T>
    ) -> U where T:Copy{
        let mut guard = self.0.lock().unwrap();
        let len = guard.len();
        if count <= len {
            guard.pop_blocking(count, consumer)
        } else {
            guard.pop_blocking(len,|first:&[T],second:&[T]|{
                let data:Vec<_> = first.iter().cloned().chain(second.iter().cloned()).collect();
                let padding:Vec<_> = producer.take(count-len).collect();
                let (value,_) = consumer(&data,&padding);
                (value,len)
            })
        }
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
        if guard.capacity() - guard.len() >= self.push_len_required {
            let push_fn: PushCallback = unsafe { std::mem::transmute_copy(&self.push_fn) };
            guard.push_blocking(push_fn);
            guard.push_waker.pop_front();
            let pop_waker = guard.pop_waker.pop_front();
            if let Some(pop_waker) = pop_waker {
                pop_waker.wake();
            }
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
            guard.pop_waker.pop_front();
            let push_waker = guard.push_waker.pop_front();
            if let Some(push_waker) = push_waker {
                push_waker.wake();
            }
            Poll::Ready(result)
        } else {
            guard.pop_waker.push_back(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<T, const N: usize> Default for BlockingRingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
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
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_timeout() {
        let buffer = Arc::new(RingBuffer::<f32, 1000000>::new());
        let buffer_for_consumer = buffer.clone();
        let buffer_for_producer = buffer;
        std::thread::spawn(move || {
            let array = vec![3.0; 4];
            let push = buffer_for_consumer.push(4, |par1, par2| {
                par1[..array.len()].copy_from_slice(&array);
                array.len()
            });
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            std::thread::sleep(std::time::Duration::from_secs(1));
            rt.block_on(push);
        });
        let pop = buffer_for_producer.pop(2, |par1, par2| (par1[0], 1));
        let timeout = tokio::time::timeout(std::time::Duration::from_millis(1000), pop);
        match timeout.await {
            Ok(value) => {
                println!("value is {}", value);
            }
            Err(_) => {
                println!("timeout");
            }
        }
    }
}
