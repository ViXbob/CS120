pub trait Buffer<Data>: Send + Sync {
    /// push the data into buffer, the process will be blocking when there is no space in the storage.
    fn push(&self, count: usize, producer: impl FnMut(&mut [Data], &mut [Data]));

    fn push_by_ref(&self, data: &[Data])
    where
        Data: Copy + Clone,
    {
        self.push(data.len(), |first, second| {
            data.iter()
                .zip(first.iter_mut().chain(second.iter_mut()))
                .for_each(|(data, store)| {
                    *store = *data;
                })
        })
    }

    fn push_by_iterator(&self, count: usize, data: &mut impl Iterator<Item = Data>)
    where
        Data: Copy + Clone,
    {
        self.push(count, |first, second| {
            data.zip(first.iter_mut().chain(second.iter_mut()))
                .for_each(|(data, store)| {
                    *store = data;
                })
        })
    }

    /// pop the data from the buffer, the data will be removed after the consumer call
    fn pop<T>(&self, count: usize, consumer: impl FnMut(&[Data], &[Data]) -> T) -> T;

    fn pop_by_ref<T>(&self, count: usize, mut consumer: impl FnMut(&[Data]) -> T) -> T
    where
        Data: Copy + Clone,
    {
        self.pop(count, |first, second| {
            let slice = [first, second].concat();
            consumer(&slice)
        })
    }

    fn pop_by_iterator<'a, T>(
        &self,
        count: usize,
        mut consumer: impl FnMut(Box<dyn Iterator<Item = &Data> + '_>) -> T,
    ) -> T
    where
        Data: std::clone::Clone,
    {
        self.pop(count, |first, second| {
            let b = first.iter().chain(second.iter());
            let value = consumer(Box::new(b));
            value
        })
    }
}
