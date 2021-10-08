use cpal::SampleFormat as CpalSampleFormat;
use hound::SampleFormat as HoundSampleFormat;
use hound::WavSpec;
use std::mem;

#[derive(Copy, Clone, Debug)]
pub struct SoundDescriptor {
    pub channels: u16,
    pub sample_rate: u32,
    pub sample_format: SampleFormat,
}

#[derive(Copy, Clone, Debug)]
pub enum SampleFormat {
    /// The value 0 corresponds to 0.
    I16,
    /// The value 0 corresponds to 32768.
    U16,
    /// The boundaries are (-1.0, 1.0).
    F32,
}

impl SampleFormat {
    fn sample_size(&self) -> usize {
        match self {
            SampleFormat::I16 => mem::size_of::<i16>() * 8,
            SampleFormat::U16 => mem::size_of::<u16>() * 8,
            SampleFormat::F32 => mem::size_of::<f32>() * 8,
        }
    }
}

impl From<CpalSampleFormat> for SampleFormat {
    fn from(format: CpalSampleFormat) -> Self {
        match format {
            CpalSampleFormat::I16 => SampleFormat::I16,
            CpalSampleFormat::U16 => SampleFormat::U16,
            CpalSampleFormat::F32 => SampleFormat::F32,
        }
    }
}

#[allow(clippy::from_over_into)]
impl Into<HoundSampleFormat> for SampleFormat {
    fn into(self) -> HoundSampleFormat {
        match self {
            SampleFormat::I16 => HoundSampleFormat::Int,
            SampleFormat::U16 => HoundSampleFormat::Int,
            SampleFormat::F32 => HoundSampleFormat::Float,
        }
    }
}

impl From<HoundSampleFormat> for SampleFormat {
    fn from(format: HoundSampleFormat) -> Self {
        match format {
            HoundSampleFormat::Int => SampleFormat::I16,
            HoundSampleFormat::Float => SampleFormat::F32,
        }
    }
}
#[allow(clippy::from_over_into)]
impl Into<WavSpec> for SoundDescriptor {
    fn into(self) -> WavSpec {
        WavSpec {
            channels: self.channels,
            sample_rate: self.sample_rate,
            bits_per_sample: self.sample_format.sample_size() as u16,
            sample_format: self.sample_format.into(),
        }
    }
}
