use cs140_common::audio_buffer::AudioData as AData;

use cpal::Sample;
use std::mem::MaybeUninit;

pub struct AudioData<const N: usize>([f32; N]);

impl<const N: usize> Copy for AudioData<N> {}

impl<const N: usize> Clone for AudioData<N> {
    fn clone(&self) -> Self {
        let mut data: [f32; N] =
            unsafe { MaybeUninit::array_assume_init(std::mem::MaybeUninit::uninit_array()) };
        unsafe {
            std::ptr::copy_nonoverlapping(&self.0, &mut data as *mut [f32; N], N);
        }
        AudioData { 0: data }
    }
}

impl<const N: usize> AData for AudioData<N> {
    fn from_sample_slice<SampleType: Sample>(sample_input: &[SampleType]) -> Self {
        assert_eq!(sample_input.len(), N);
        let data = unsafe {
            let mut data: [MaybeUninit<f32>; N] = MaybeUninit::uninit_array();
            sample_input
                .into_iter()
                .enumerate()
                .for_each(|(index, value)| {
                    data[index].write(value.to_f32());
                });
            MaybeUninit::array_assume_init(data)
        };
        AudioData { 0: data }
    }

    fn iter<SampleType: Sample>(self) -> Box<dyn Iterator<Item = SampleType>> {
        return Box::new(self.0.into_iter().map(|x| SampleType::from(&x)));
    }
}
