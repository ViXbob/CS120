use std::cmp::min;
use super::header;
use crate::encoding::{BitStore, encode_4b5b, decode_4b5b};
use bitvec::vec::BitVec;
use log::{debug, trace, warn};
use rand::seq::index::sample;
use rustfft::{num_complex::Complex, FftPlanner};
use rustfft::num_traits::Pow;
use rustfft::num_traits::real::Real;

const SCALE: f32 = 1.0;
const RECOVERY_RANGE: usize = 5;

pub fn frame_resolve_to_bitvec(
    data: &[f32],
    header: &[f32],
    sample_per_bit: usize,
    frame_length: usize,
) -> (Option<Vec<BitStore>>, usize) {
    let begin_index = header::detect_header(data.iter(), header); //.expect("detection failed");
    if begin_index.is_none() {
        return (None, data.len() - header.len());
    }
    let mut begin_index = begin_index.unwrap();
    trace!("begin_index: {}", begin_index);
    let frame_length: usize = frame_length / 4 * 5;
    let shift: usize = min(RECOVERY_RANGE, begin_index);
    begin_index -= shift;
    if begin_index + frame_length * (sample_per_bit as usize) + sample_per_bit >= data.len() {
        return (None, begin_index - header.len() * 2);
    }
    let correlation = |data: &[f32]| -> f32 {
        let mut rnt: f32 = 0.0;
        let mut mid: f32 = (sample_per_bit as f32 - 1.0) / 2.0;
        for (index, value) in data.iter().enumerate() {
            rnt += value * (-(index as f32 - mid).pow(2 as i32)).exp();
        }
        rnt
    };
    let mut rnt: Vec<BitStore> = Vec::new();
    for begin_index in begin_index..begin_index + 2 * shift + 1 {
        let mut result: BitStore = BitVec::new();
        if begin_index + frame_length * (sample_per_bit as usize) + sample_per_bit >= data.len() {
            break;
        } else {
            let mut pre_bit: bool = correlation(&data[begin_index..begin_index + sample_per_bit]) > 0.0;
            let mut pre_sample: f32 = data[begin_index + 1];
            if !pre_bit { continue; }
            debug!("first  bit: {}", pre_bit);
            for samples in data[begin_index + sample_per_bit..begin_index + sample_per_bit * (frame_length + 1)].chunks(sample_per_bit) {
                let value = correlation(samples);
                // trace!("{}", value);
                let mut bit: bool = value > 0.0;
                if sample_per_bit == 2 {
                    if samples[0] < 0.0 && samples[1] > 0.0 {
                        bit = pre_sample < 0.0;
                    } else if samples[0] > 0.0 && samples[1] < 0.0 {
                        bit = pre_sample > 0.0;
                    }
                    pre_sample = samples[1];
                }
                result.push(bit != pre_bit);
                pre_bit = bit;
            }
            rnt.push(decode_4b5b(&result));
        }
    }
    (
        Some(rnt),
        (begin_index + (frame_length + 1) * sample_per_bit as usize) as usize,
    )
}

pub fn generate_frame_sample_from_bitvec(
    data: &BitStore,
    header: &[f32],
    sample_per_bit: usize,
) -> Vec<f32> {
    let data = encode_4b5b(data);
    let mut rnt: Vec<f32> = header.to_owned();
    let trans_bit_to_sample = |x : bool| -> Vec<f32> {
        (0..sample_per_bit).map(|_ : _| -> f32 { if x { 1.0 * SCALE } else { -1.0 * SCALE } }).collect()
    };
    rnt.extend(trans_bit_to_sample(true).iter());
    let mut pre_bit: bool = true;
    for bit in data {
        if bit {
            pre_bit = !pre_bit;
            rnt.extend(trans_bit_to_sample(pre_bit).iter());
        } else {
            rnt.extend(trans_bit_to_sample(pre_bit).iter());
        }
    }
    rnt
}