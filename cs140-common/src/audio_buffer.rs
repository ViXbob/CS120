use cpal::Sample;

pub trait AudioData: Copy {
    fn from_sample_slice<SampleType: Sample>(sample_input: &[SampleType]) -> Self;
    fn iter<SampleType: Sample>(self) -> Box<dyn Iterator<Item = SampleType>>;
}

pub trait AudioBuffer: Send + Sync {
    type Data: AudioData;

    /// push the data into buffer, the process will be blocking when there is no space in the storage.
    fn push(&self, data: Self::Data);
    /// pop the data from the buffer, the data will be removed after the consumer call
    fn pop<T>(&self, consumer: impl FnMut(&Self::Data) -> T) -> T;
}
