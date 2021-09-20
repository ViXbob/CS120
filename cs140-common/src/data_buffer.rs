use std::marker::{Send, Sync};

pub trait DataBuffer: Send + Sync {
    type Data: Sized;

    /// push the data into buffer, the process will be blocking when there is no space in the storage.
    fn push(&self, data: Self::Data);
    /// pop the data from the buffer, the data will be removed after the consumer call
    fn pop<T>(&self, consumer: impl FnMut(&Self::Data) -> T) -> T;
}
