use std::slice::from_raw_parts_mut;
use std::sync::{Arc, Mutex};
use cs140_common::buffer::Buffer;

pub struct VecBuffer<T>{
  data:Arc<Mutex<Vec<T>>>,
    head:Arc<Mutex<usize>>,
}

impl<T> VecBuffer<T>{
    pub fn new() ->Self{
        VecBuffer{
            data:Default::default(),
            head:Default::default(),
        }
    }
}

impl<T:Send> Buffer<T> for VecBuffer<T>{
    fn push(&self, count: usize, producer: impl FnOnce(&mut [T], &mut [T]) -> usize) {
        let mut guard = self.data.lock().unwrap();
        let head = self.head.lock().unwrap();
        guard.reserve(count);
        unsafe{
            let data_ptr = guard.as_ptr();
            let data_ptr = data_ptr as *mut T;
            producer(from_raw_parts_mut(data_ptr,guard.len() - *head),&mut[]);
        };
    }

    fn try_push(&self, count: usize, producer: impl FnOnce(&mut [T], &mut [T]) -> usize) -> Option<()> {
        Some(self.push(count,producer))
    }

    fn pop<U>(&self, count: usize, consumer: impl FnOnce(&[T], &[T]) -> (U, usize)) -> U {
        loop{
            let mut guard = self.data.lock().unwrap();
            let mut head = self.head.lock().unwrap();
            if guard.len() - *head < count{
                std::thread::sleep(std::time::Duration::from_micros(20));
                continue
            }
            let (value,count) =  consumer(&guard[head.clone()..head.clone() + count],&[]);
             *head += count;
            return value;
        }
    }

    fn try_pop<U>(&self, count: usize, consumer: impl FnOnce(&[T], &[T]) -> (U, usize)) -> Option<U> {
        Some(self.pop(count,consumer))
    }
}