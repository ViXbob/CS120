use async_trait::async_trait;

#[async_trait]
pub trait Buffer<Data: Send + Sync>: Send + Sync {
    /// push the data into buffer, the process will be blocking when there is no space in the storage.
    async fn push(
        &self,
        count: usize,
        producer: impl for<'a> FnOnce(&'a mut [Data], &'a mut [Data]) -> usize + Send + 'async_trait,
    );
    async fn push_by_ref(&self, data: &[Data])
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
        })
        .await;
    }

    async fn push_by_iterator(&self, count: usize, data: &mut (impl Iterator<Item = Data> + Send))
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
        })
        .await;
    }

    /// pop the data from the buffer, the data will be removed after the consumer call
    async fn pop<T>(
        &self,
        count: usize,
        consumer: impl for<'a> FnOnce(&'a [Data], &'a [Data]) -> (T, usize) + Send + 'async_trait,
    ) -> T;
    fn must_pop<U>(
        &self,
        count: usize,
        consumer: impl FnOnce(&[Data], &[Data]) -> (U, usize),
        producer: impl Iterator<Item = Data>,
    ) -> U;
    async fn pop_by_ref<T>(
        &self,
        count: usize,
        consumer: impl for<'a> FnOnce(&'a [Data]) -> (T, usize) + Send + 'async_trait,
    ) -> T
    where
        Data: Copy + Clone,
    {
        self.pop(count, |first, second| {
            let slice = [first, second].concat();
            consumer(&slice)
        })
        .await
    }

    async fn pop_by_iterator<T>(
        &self,
        count: usize,
        consumer: impl for<'b> FnOnce(Box<dyn Iterator<Item = &'b Data> + '_>) -> (T, usize)
            + Send
            + 'async_trait,
    ) -> T
    where
        Data: std::clone::Clone,
    {
        self.pop(count, |first, second| {
            let b = first.iter().chain(second.iter());
            consumer(Box::new(b))
        })
        .await
    }
}
