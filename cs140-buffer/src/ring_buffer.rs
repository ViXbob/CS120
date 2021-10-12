use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::Mutex;
use std::thread;
use std::thread::Thread;

use std::mem::MaybeUninit;

pub struct RingBuffer<T, const N: usize, const GARBAGE_COLLECTION: bool> {
    buffer: Box<[T;N]>,
    head: AtomicUsize,
    len: AtomicUsize,
    blocking_reader: Mutex<Option<(Thread, usize)>>,
    blocking_writer: Mutex<Option<(Thread, usize)>>,
}

impl<T, const N: usize, const GARBAGE_COLLECTION: bool> Default
for RingBuffer<T, N, GARBAGE_COLLECTION>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T, const N: usize, const GARBAGE_COLLECTION: bool> RingBuffer<T, N, GARBAGE_COLLECTION> {
    #[allow(clippy::uninit_assumed_init)]
    pub fn new() -> Self {
        RingBuffer {
            buffer: unsafe { Box::new_uninit().assume_init() },
            head: AtomicUsize::new(0),
            len: AtomicUsize::new(0),
            blocking_reader: Mutex::new(None),
            blocking_writer: Mutex::new(None),
        }
    }

    fn len(&self) -> usize {
        self.len.load(Relaxed)
    }

    pub fn capacity(&self) -> usize {
        N
    }

    fn get_available_count(&self) -> usize {
        N - self.len()
    }

    pub fn try_push(
        &self,
        count: usize,
        producer: impl FnOnce(&mut [T], &mut [T]) -> usize,
    ) -> Option<()> {
        if count <= self.get_available_count() {
            self.push(count, producer);
            Some(())
        } else {
            None
        }
    }

    pub fn push(&self, count: usize, producer: impl FnOnce(&mut [T], &mut [T]) -> usize) {
        assert_ne!(count,0);
        if count > N / 2 {
            panic!("Can not push more than {} elements", N / 2);
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
            drop(self.blocking_writer.lock().unwrap()); // to ensure that blocking_writer is set to None
        }
        let head = self.head.load(Relaxed);
        let tail = (head + self.len()) % N;
        // This is safe because only one thread can access the variable
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
                let ptr = self.buffer[tail..head].as_ptr();
                let ptr = ptr as *mut T;
                producer(std::slice::from_raw_parts_mut(ptr, head - tail), &mut [])
            }
        };

        self.len.fetch_add(count, Relaxed);

        let mut lock_guard = self.blocking_reader.lock().unwrap();
        if let Some((reader_thread, need_count)) = lock_guard.as_ref() {
            if *need_count <= self.len() {
                reader_thread.unpark();
                *lock_guard = None;
            }
        }
    }

    pub fn try_pop<U>(
        &self,
        count: usize,
        consumer: impl FnOnce(&[T], &[T]) -> (U, usize),
    ) -> Option<U> {
        if count <= self.len() {
            Some(self.pop(count, consumer))
        } else {
            None
        }
    }

    pub fn pop<U>(&self, count: usize, consumer: impl FnOnce(&[T], &[T]) -> (U, usize)) -> U {
        if count == 0 {
            return consumer(&[], &[]).0;
        }
        if count > N / 2 {
            panic!("Can not acquire more than {} elements", N / 2);
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
            drop(self.blocking_reader.lock().unwrap()); // to ensure that blocking_writer is set to None
        }

        let head = self.head.load(Relaxed);
        let tail = (head + self.len()) % N;
        // This is safe because only one thread can access the variable
        let (result, count) = unsafe {
            if head < tail {
                let ptr = self.buffer[head..tail].as_ptr();
                let ptr = ptr as *mut T;
                let slice = std::slice::from_raw_parts_mut(ptr, count);

                let (value, count) = consumer(slice, &[]);
                if GARBAGE_COLLECTION {
                    (0..count).for_each(|i| std::ptr::drop_in_place(&mut slice[i]));
                }
                (value, count)
            } else {
                let first_ptr = self.buffer[head..].as_ptr();
                let first_ptr = first_ptr as *mut T;
                let mut first_slice = std::slice::from_raw_parts_mut(first_ptr, N - head);
                if first_slice.len() >= count {
                    first_slice = &mut first_slice[..count];
                    let (value, count) = consumer(first_slice, &[]);
                    if GARBAGE_COLLECTION {
                        (0..count).for_each(|i| std::ptr::drop_in_place(&mut first_slice[i]));
                    }
                    (value, count)
                } else {
                    let second_ptr = self.buffer.as_ptr(); // this pointer is started from zero
                    let second_ptr = second_ptr as *mut T;
                    let second_slice =
                        std::slice::from_raw_parts_mut(second_ptr, count - first_slice.len());
                    let (value, count) = consumer(first_slice, second_slice);
                    if GARBAGE_COLLECTION {
                        if count <= first_slice.len() {
                            (0..count).for_each(|i| std::ptr::drop_in_place(&mut first_slice[i]));
                        } else {
                            (0..first_slice.len())
                                .for_each(|i| std::ptr::drop_in_place(&mut first_slice[i]));

                            (0..count - first_slice.len())
                                .for_each(|i| std::ptr::drop_in_place(&mut second_slice[i]));
                        }
                    }
                    (value, count)
                }
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

impl<T, const N: usize, const GARBAGE_COLLECTION: bool> Drop
for RingBuffer<T, N, GARBAGE_COLLECTION>
{
    fn drop(&mut self) {
        if GARBAGE_COLLECTION {
            let head = self.head.load(Relaxed);
            let tail = (self.len() + head) % N;
            (head..{
                if head > tail {
                    tail + N
                } else {
                    tail
                }
            })
                .for_each(|index| unsafe { std::ptr::drop_in_place(&mut self.buffer[index % N]) });
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use cs140_common::buffer::Buffer;
    use std::sync::Arc;
    use std::thread;

    use std::time::Instant;
    use crate::vec_buffer::VecBuffer;

    #[test]
    fn test_ring_buffer() {
        let buffer = Arc::new(RingBuffer::<f32, 1000000, false>::new());
        let buffer_for_consumer = buffer.clone();
        let buffer_for_producer = buffer;
        let consumer = thread::spawn(move || {
            for _ in 0..1000 {
                let start = Instant::now();
                buffer_for_consumer.pop_by_ref(4000, |_| ((), 4000));
                println!("pop cost: {} ns", start.elapsed().as_nanos())
            }
            loop {
                if buffer_for_consumer.try_pop(1000,|_,_|((),1000)).is_none(){
                    break;
                }
            }
        });
        let producer = thread::spawn(move || {
            for _ in 0..100 {
                let data: Vec<_> = (0..40000).map(|x| (x as f32).sin()).collect();
                let start = Instant::now();
                buffer_for_producer.push_by_ref(&data);
                println!("push cost: {} ns", start.elapsed().as_nanos())
            }
        });
        consumer.join().unwrap();
        producer.join().unwrap();
    }
}
