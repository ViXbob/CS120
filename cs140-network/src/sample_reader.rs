use log::trace;
use crate::encoding::BitStore;

static BIT_SLIP_HISTORY_COUNT: usize = 4;
static SAMPLE_PER_BIT: usize = 2;
static EXPONENTIALLY_WEIGHTED_MOVING_AVERAGE_NEW_DATA_RATIO: f32 = 0.5;
static ZERO_RANGE: f32 = ACCEPTABLE_NO_OFFSET_SIGNAL_RANGE;
// check the sample is in 50% range of zero
static ACCEPTABLE_NO_OFFSET_SIGNAL_RANGE: f32 = 0.5;

#[derive(Debug, Copy, Clone)]
pub struct SampleReader {
    one_amplitude: f32,
    zero_amplitude: f32,
    neg_one_amplitude: f32,
    bit_slip_history: usize,
}

impl SampleReader {
    pub fn new(zero_amplitude: f32, one_amplitude: f32, neg_one_amplitude: f32) -> Self {
        Self {
            zero_amplitude,
            one_amplitude,
            neg_one_amplitude,
            bit_slip_history: 0,
        }
    }

    pub fn read_all(&mut self, data: &[f32]) -> (BitStore, usize) {
        let mut result = BitStore::with_capacity(data.len() / 2);
        let mut data_ref = data;
        loop {
            let bit = self.read(&mut data_ref);
            match bit {
                None => { return (result, data.len() - data_ref.len()); }
                Some(bit) => { result.push(bit); }
            }
            println!("{}",result);
        }
    }

    fn read(&mut self, data: &mut &[f32]) -> Option<bool> {
        // if this assertion fails, please check the count of your max package size with the count of samples that pop from buffer
        assert!(data.len() > SAMPLE_PER_BIT);

        let current_bit_sample = &data[..SAMPLE_PER_BIT];

        if current_bit_sample.iter().all(|sample| {
            (*sample - self.zero_amplitude).abs() < ZERO_RANGE * (self.one_amplitude - self.zero_amplitude)
        }) {
            return None;
        }

        let current_bit_average_value = current_bit_sample.iter().sum::<f32>() / current_bit_sample.len() as f32;

        let result = current_bit_average_value > self.zero_amplitude;

        let (current_bit_max_amplitude_index, current_bit_max_amplitude) = current_bit_sample.iter().map(|x| x.abs()).enumerate().fold((SAMPLE_PER_BIT, 0f32), |old_max, (index, abs_value)| {
            return if abs_value > old_max.1 {
                (index, abs_value)
            } else {
                old_max
            };
        });
        assert_ne!(current_bit_max_amplitude_index, SAMPLE_PER_BIT);

        let (current_bit_min_amplitude_index, current_bit_min_amplitude) = current_bit_sample.iter().map(|x| x.abs()).enumerate().fold((SAMPLE_PER_BIT, 1.0), |old_min, (index, abs_value)| {
            return if abs_value < old_min.1 {
                (index, abs_value)
            } else {
                old_min
            };
        });
        assert_ne!(current_bit_min_amplitude_index, SAMPLE_PER_BIT);

        // update 1 and -1
        if result {
            self.one_amplitude = self.one_amplitude * (1.0 - EXPONENTIALLY_WEIGHTED_MOVING_AVERAGE_NEW_DATA_RATIO) + current_bit_max_amplitude * EXPONENTIALLY_WEIGHTED_MOVING_AVERAGE_NEW_DATA_RATIO;
        } else {
            self.neg_one_amplitude = self.neg_one_amplitude * (1.0 - EXPONENTIALLY_WEIGHTED_MOVING_AVERAGE_NEW_DATA_RATIO) - current_bit_max_amplitude * EXPONENTIALLY_WEIGHTED_MOVING_AVERAGE_NEW_DATA_RATIO;
        }

        if current_bit_min_amplitude_index != 0 && current_bit_min_amplitude_index != SAMPLE_PER_BIT - 1 {
            // if assertion failed, please check the signal in Au.
            assert!(self.check_sample_is_acceptable(current_bit_sample, result));
            // the bit is flawless
            *data = &data[SAMPLE_PER_BIT..];
            return Some(result);
        }

        if self.check_sample_is_acceptable(current_bit_sample, result) {
            // the bit is flawless
            *data = &data[SAMPLE_PER_BIT..];
            return Some(result);
        }

        if self.bit_slip_history != 0 {
            self.bit_slip_history -= 1;
            *data = &data[SAMPLE_PER_BIT..];
        } else {
            if (current_bit_sample[current_bit_min_amplitude_index] + self.zero_amplitude) * (current_bit_sample[current_bit_max_amplitude_index] + self.zero_amplitude) < 0.0 {
                self.bit_slip_history = BIT_SLIP_HISTORY_COUNT;
                if current_bit_min_amplitude_index == 0 {
                    *data = &data[SAMPLE_PER_BIT + 1..];
                } else {
                    *data = &data[SAMPLE_PER_BIT - 1..];
                }
            } else {
                *data = &data[SAMPLE_PER_BIT..];
            }
        }
        Some(result)
    }

    fn check_sample_is_acceptable(&self, current_bit_sample: &[f32], result: bool) -> bool {
        current_bit_sample.iter().all(|sample| {
            if result {
                *sample > (1.0 - ACCEPTABLE_NO_OFFSET_SIGNAL_RANGE) * self.zero_amplitude + ACCEPTABLE_NO_OFFSET_SIGNAL_RANGE * self.one_amplitude
            } else {
                *sample < (1.0 - ACCEPTABLE_NO_OFFSET_SIGNAL_RANGE) * self.zero_amplitude + ACCEPTABLE_NO_OFFSET_SIGNAL_RANGE * self.neg_one_amplitude
            }
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub struct ZeroReader {
    one_amplitude: f32,
    zero_amplitude: f32,
    neg_one_amplitude: f32,
}

impl ZeroReader {
    pub fn new() -> Self {
        ZeroReader {
            one_amplitude: 0.1,
            zero_amplitude: 0.0,
            neg_one_amplitude: -0.1,
        }
    }

    pub fn read_all(&mut self, data: &[f32]) -> usize {
        let mut index = 0;
        while index < data.len() {
            if (data[index] + self.zero_amplitude).abs() < ZERO_RANGE * (self.one_amplitude - self.zero_amplitude) {
                index += 1;
            } else {
                trace!("Sample value: {}, signal found",data[index]);
                break;
            }
        }
        index
    }
}

impl From<SampleReader> for ZeroReader {
    fn from(reader: SampleReader) -> Self {
        Self {
            one_amplitude: reader.one_amplitude,
            zero_amplitude: reader.zero_amplitude,
            neg_one_amplitude: reader.neg_one_amplitude,
        }
    }
}

impl From<ZeroReader> for SampleReader {
    fn from(reader: ZeroReader) -> Self {
        Self {
            one_amplitude: reader.one_amplitude,
            zero_amplitude: reader.zero_amplitude,
            neg_one_amplitude: reader.neg_one_amplitude,
            bit_slip_history: 0,
        }
    }
}
