pub trait Buffer<Data>: Send + Sync where Data: Copy {
    /// push the data into buffer, the process will be blocking when there is no space in the storage.
    fn push(&self, count: usize, producer: impl FnOnce(&mut [Data], &mut [Data]) -> usize);
    fn try_push(
        &self,
        count: usize,
        producer: impl FnOnce(&mut [Data], &mut [Data]) -> usize,
    ) -> Option<()>;
    fn push_by_ref(&self, data: &[Data])
        where
            Data: Copy + Clone,
    {
        self.push(data.len(), |first, second| {
            data.iter()
                .zip(first.iter_mut().chain(second.iter_mut()))
                .for_each(|(data, store)| {
                    *store = *data;
                });
            data.len()
        });
    }

    fn push_by_iterator(&self, count: usize, data: &mut impl Iterator<Item=Data>)
        where
            Data: Copy + Clone,
    {
        self.push(count, |first, second| {
            let mut count = 0;
            data.zip(first.iter_mut().chain(second.iter_mut()))
                .for_each(|(data, store)| {
                    *store = data;
                    count += 1;
                });
            count
        });
    }

    /// pop the data from the buffer, the data will be removed after the consumer call
    fn pop<T>(&self, count: usize, consumer: impl FnOnce(&[Data], &[Data]) -> (T, usize)) -> T;
    fn must_pop<T> (
        &self,
        count: usize,
        consumer: impl FnOnce(&[Data], &[Data]) -> (T, usize),
        producer: impl Iterator<Item=Data>,
    ) -> T;
    fn pop_by_ref<T>(&self, count: usize, consumer: impl FnOnce(&[Data]) -> (T, usize)) -> T
        where
            Data: Copy + Clone,
    {
        self.pop(count, |first, second| {
            let slice = [first, second].concat();
            consumer(&slice)
        })
    }

    fn pop_by_iterator<T>(
        &self,
        count: usize,
        consumer: impl FnOnce(Box<dyn Iterator<Item=&Data> + '_>) -> (T, usize),
    ) -> T
        where
            Data: std::clone::Clone,
    {
        self.pop(count, |first, second| {
            let b = first.iter().chain(second.iter());
            consumer(Box::new(b))
        })
    }
}
