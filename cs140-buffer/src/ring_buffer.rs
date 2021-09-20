use cs140_common::data_buffer::DataBuffer;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Mutex;
use std::thread;
use std::thread::Thread;

use std::mem::MaybeUninit;

pub struct RingBuffer<T, const N: usize> {
    buffer: [T; N],
    head: AtomicUsize,
    tail: AtomicUsize,
    blocking_reader: Mutex<Option<Thread>>,
    blocking_writer: Mutex<Option<Thread>>,
}

impl<T, const N: usize> RingBuffer<T, N> {
    fn new() -> Self {
        RingBuffer {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            head: AtomicUsize::new(0),
            tail: AtomicUsize::new(0),
            blocking_reader: Mutex::new(None),
            blocking_writer: Mutex::new(None),
        }
    }

    pub fn push(&self, data: T) {
        let mut parked = false;
        {
            let mut lock_guard = self.blocking_writer.lock().unwrap();
            if (self.tail.load(Relaxed) + 1) % N == self.head.load(Relaxed) {
                if lock_guard.is_some() {
                    panic!("RingBuffer is intend to used in two threads.");
                }
                *lock_guard = Some(thread::current());
                parked = true;
            }
        }
        if parked {
            thread::park();
            self.blocking_writer.lock().unwrap(); // to ensure that blocking_writer is set to None
        }
        let tail = self.tail.load(Relaxed);
        // This is safe because only one thread can access the variable
        unsafe {
            let ptr = &self.buffer[tail] as *const T;
            let mut_ptr = ptr as *mut T;
            mut_ptr.write(data);
        }
        self.tail.store((tail + 1) % N, Relaxed);

        let mut lock_guard = self.blocking_reader.lock().unwrap();
        if let Some(reader_thread) = lock_guard.as_ref() {
            reader_thread.unpark();
        }
        *lock_guard = None;
    }

    pub fn pop<U>(&self, mut consumer: impl FnMut(&T) -> U) -> U {
        let mut parked = false;
        {
            let mut lock_guard = self.blocking_reader.lock().unwrap();
            if self.tail.load(Relaxed) == self.head.load(Relaxed) {
                if lock_guard.is_some() {
                    panic!("RingBuffer is intend to used in two threads.");
                }
                *lock_guard = Some(thread::current());
                parked = true;
            }
        }
        if parked {
            thread::park();
            self.blocking_reader.lock().unwrap(); // to ensure that blocking_writer is set to None
        }
        let head = self.head.load(Relaxed);
        // This is safe because only one thread can access the variable
        let result = unsafe {
            let some = &self.buffer[head];
            let value = consumer(some);
            let ptr = &self.buffer[head] as *const T;
            let ptr = ptr as *mut T;
            std::ptr::drop_in_place(ptr);
            value
        };
        self.head.store((head + 1) % N, Relaxed);

        let mut lock_guard = self.blocking_writer.lock().unwrap();
        if let Some(write_thread) = lock_guard.as_ref() {
            write_thread.unpark();
        }
        *lock_guard = None;
        result
    }
}

impl<T, const N: usize> Drop for RingBuffer<T, N> {
    fn drop(&mut self) {
        let head = self.head.load(Relaxed);
        let tail = self.tail.load(Relaxed);
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
    use std::sync::Arc;
    use std::thread;

    #[test]
    fn test_ring_buffer() {
        let buffer = Arc::new(RingBuffer::<[f32; 32], 32>::new());
        let buffer_for_consumer = buffer.clone();
        let buffer_for_producer = buffer.clone();
        let consumer = thread::spawn(move || {
            println!(
                "{}",
                buffer_for_consumer.pop(|data| { data.iter().sum::<f32>() })
            )
        });
        let producer = thread::spawn(move || {
            buffer_for_producer.push([1.0; 32]);
            println!("pushed.");
        });
        consumer.join();
        producer.join();
    }
}
