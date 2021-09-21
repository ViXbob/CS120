use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Mutex;
use std::thread;
use std::thread::Thread;

use std::mem::MaybeUninit;

pub struct RingBuffer<T, const N: usize> {
    buffer: [T; N],
    head: AtomicUsize,
    len: AtomicUsize,
    blocking_reader: Mutex<Option<(Thread, usize)>>,
    blocking_writer: Mutex<Option<(Thread, usize)>>,
}

impl<T, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self {
        RingBuffer {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            head: AtomicUsize::new(0),
            len: AtomicUsize::new(0),
            blocking_reader: Mutex::new(None),
            blocking_writer: Mutex::new(None),
        }
    }

    fn len(&self) -> usize {
        self.len.load(Relaxed)
    }

    fn get_available_count(&self) -> usize {
        N - self.len()
    }

    pub fn push(&self, count: usize, mut producer: impl FnMut(&mut [T], &mut [T])) {
        if count == 0 {
            return;
        }
        if count > N {
            panic!("Can not push more than {} elements", N);
        }
        let mut parked = false;
        {
            let mut lock_guard = self.blocking_writer.lock().unwrap();
            let capacity = self.get_available_count();
            if capacity < count {
                if lock_guard.is_some() {
                    panic!("RingBuffer is intend to used in two threads.");
                }
                *lock_guard = Some((thread::current(), count));
                parked = true;
            }
        }
        if parked {
            thread::park();
            self.blocking_writer.lock().unwrap(); // to ensure that blocking_writer is set to None
        }
        let head = self.head.load(Relaxed);
        let tail = (head + self.len()) % N;
        // This is safe because only one thread can access the variable
        unsafe {
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
                let ptr = self.buffer[tail..head].as_ptr();
                let ptr = ptr as *mut T;
                producer(std::slice::from_raw_parts_mut(ptr, head - tail), &mut [])
            }
        }

        self.len.fetch_add(count, Relaxed);

        let mut lock_guard = self.blocking_reader.lock().unwrap();
        if let Some((reader_thread, need_count)) = lock_guard.as_ref() {
            if *need_count <= self.len() {
                reader_thread.unpark();
                *lock_guard = None;
            }
        }
    }

    pub fn pop<U>(&self, count: usize, mut consumer: impl FnMut(&[T], &[T]) -> U) -> U {
        if count == 0 {
            return consumer(&[], &[]);
        }
        if count > N {
            panic!("Can not acquire more than {} elements", N);
        }
        let mut parked = false;
        {
            let mut lock_guard = self.blocking_reader.lock().unwrap();
            let len = self.len();
            if len < count {
                if lock_guard.is_some() {
                    panic!("RingBuffer is intend to used in two threads.");
                }
                *lock_guard = Some((thread::current(), count));
                parked = true;
            }
        }
        if parked {
            thread::park();
            self.blocking_reader.lock().unwrap(); // to ensure that blocking_writer is set to None
        }

        let head = self.head.load(Relaxed);
        let tail = (head + self.len()) % N;
        // This is safe because only one thread can access the variable
        let result = unsafe {
            if head < tail {
                let ptr = self.buffer[head..tail].as_ptr();
                let ptr = ptr as *mut T;
                let slice = std::slice::from_raw_parts_mut(ptr, tail-head);

                let value = consumer(slice, &[]);

                (0..slice.len()).for_each(|i| std::ptr::drop_in_place(&mut slice[i]));
                value
            } else {
                let first_ptr = self.buffer[head..].as_ptr();
                let first_ptr = first_ptr as *mut T;
                let first_slice = std::slice::from_raw_parts_mut(first_ptr, N - head);
                let second_ptr = self.buffer[..tail].as_ptr();
                let second_ptr = second_ptr as *mut T;
                let second_slice = std::slice::from_raw_parts_mut(second_ptr, tail);

                let value = consumer(first_slice, second_slice);
                (0..first_slice.len()).for_each(|i| std::ptr::drop_in_place(&mut first_slice[i]));

                (0..second_slice.len()).for_each(|i| std::ptr::drop_in_place(&mut second_slice[i]));
                value
            }
        };

        self.head.store((head + count) % N, Relaxed);
        self.len.fetch_sub(count, Relaxed);

        let mut lock_guard = self.blocking_writer.lock().unwrap();
        if let Some((write_thread, space_need)) = lock_guard.as_ref() {
            if *space_need <= self.get_available_count() {
                write_thread.unpark();
                *lock_guard = None;
            }
        }
        result
    }
}

impl<T, const N: usize> Drop for RingBuffer<T, N> {
    fn drop(&mut self) {
        let head = self.head.load(Relaxed);
        let tail = (self.len() + head) % N;
        (head..{
            if head > tail {
                tail + N
            } else {
                tail
            }
        })
            .for_each(|index| unsafe { std::ptr::drop_in_place(&mut self.buffer[index % N]) })
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use cs140_common::buffer::Buffer;
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_ring_buffer() {
        let buffer = Arc::new(RingBuffer::<f32, 32>::new());
        let buffer_for_consumer = buffer.clone();
        let buffer_for_producer = buffer.clone();
        let consumer = thread::spawn(move || {
            println!(
                "{}",
                buffer_for_consumer.pop_by_iterator(20, |iter| { iter.sum::<f32>() })
            )
        });
        let producer = thread::spawn(move || {
            buffer_for_producer.push_by_ref(&[1.0; 32]);
            println!("pushed.");
        });
        consumer.join();
        producer.join();
    }
}
