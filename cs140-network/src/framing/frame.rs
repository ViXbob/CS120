use super::header;
use crate::encoding::BitStore;
use bitvec::vec::BitVec;
use rustfft::{num_complex::Complex, FftPlanner};

// pub fn frame_resolve(
//     data: &[f32],
//     header_length: usize,
//     header: &[f32],
//     multiplex_range: usize,
//     multiplex_frequency: &[f32],
//     sample_rate: u32,
//     speed: u32,
//     frame_length: usize,
// ) -> Result<(Vec<i32>, usize), &'static str> {
//     let begin_index = header::detect_header(data.iter(), header).expect("detection failed");
//     let sample_per_bit = sample_rate / speed;
//     let fft_len: usize = sample_per_bit as usize;
//     let mut planner = FftPlanner::<f32>::new();
//     let fft = planner.plan_fft_forward(fft_len);
//     // let mut buffer: Vec<Complex<f32>> = Vec::new();
//     let mut result: Vec<i32> = Vec::new();
//     for i in 0..frame_length {
//         let buffer = data[(begin_index + i * sample_per_bit as usize)
//             ..(begin_index + (i + 1) * sample_per_bit as usize)]
//             .iter()
//             .map(|x: _| Complex::<f32>::new(*x, 0.0));
//         // let mut buffer : Vec<_> = buffer.skip(sample_per_bit as usize / 4).take(sample_per_bit as usize / 2).cycle().take(sample_per_bit as usize).collect();
//         let mut buffer: Vec<_> = buffer.collect();
//         fft.process(buffer.as_mut_slice());
//         for frequency in multiplex_frequency {
//             let index: usize = (*frequency as usize) / ((sample_rate / sample_per_bit) as usize);
//             let value = buffer[index];
//             println!("{}", value.im / (sample_per_bit as f32) * 2.0);
//             if (value.im.abs() / (sample_per_bit as f32) * 2.0 > 0.01) && (value.im < 0.0) {
//                 result.push(1);
//             } else {
//                 result.push(0);
//             }
//             // if (value.im.abs() / (sample_per_bit as f32) * 2.0 > 0.01) && (value.im > 0.0)
//         }
//     }
//     Ok((
//         result,
//         (begin_index + frame_length * sample_per_bit as usize) as usize,
//     ))
// }

pub fn frame_resolve_to_bitvec(
    data: &[f32],
    header: &[f32],
    multiplex_frequency: &[f32],
    sample_rate: u32,
    speed: u32,
    frame_length: usize,
) -> (Option<BitStore>, usize) {
    let begin_index = header::detect_header(data.iter(), header); //.expect("detection failed");
    if begin_index.is_none() {
        return (None, data.len() - header.len());
    }
    let begin_index = begin_index.unwrap();
    let sample_per_bit = sample_rate / speed;

    if begin_index + frame_length * (sample_per_bit as usize) >= data.len() {
        return (None, begin_index - header.len() * 2);
    }

    let fft_len: usize = sample_per_bit as usize;
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_len);
    // let mut buffer: Vec<Complex<f32>> = Vec::new();
    let mut result: BitStore = BitVec::new();
    for i in 0..frame_length {
        let buffer = data[(begin_index + i * sample_per_bit as usize)
            ..(begin_index + (i + 1) * sample_per_bit as usize)]
            .iter()
            .map(|x: _| Complex::<f32>::new(*x, 0.0));
        // let mut buffer : Vec<_> = buffer.skip(sample_per_bit as usize / 4).take(sample_per_bit as usize / 2).cycle().take(sample_per_bit as usize).collect();
        let mut buffer: Vec<_> = buffer.collect();
        fft.process(buffer.as_mut_slice());
        for frequency in multiplex_frequency {
            let index: usize = (*frequency as usize) / ((sample_rate / sample_per_bit) as usize);
            let value = buffer[index];
            // println!("{}", value.im / (sample_per_bit as f32) * 2.0);
            if (value.im.abs() / (sample_per_bit as f32) * 2.0 > 0.01) && (value.im < 0.0) {
                result.push(true);
            } else {
                result.push(false);
            }
            // if (value.im.abs() / (sample_per_bit as f32) * 2.0 > 0.01) && (value.im > 0.0)
        }
    }
    (
        Some(result),
        (begin_index + frame_length * sample_per_bit as usize) as usize,
    )
}

pub fn generate_frame_sample(
    data: &[i32],
    multiplex_range: usize,
    multiplex_frequency: &[f32],
    sample_rate: u32,
    speed: u32,
) -> Vec<f32> {
    assert!(multiplex_range > 0);
    let samples_per_bit: f32 = (sample_rate / speed) as f32;
    let mut rtn: Vec<f32> = header::create_header(220, 3000.0, 6000.0, sample_rate);
    let sample_rate: f32 = sample_rate as f32;
    for (i, bits_group) in data.chunks(multiplex_range).enumerate() {
        for time in i * (samples_per_bit as usize)..(i + 1) * (samples_per_bit as usize) {
            let phase: f32 = 2.0 * std::f32::consts::PI * time as f32 / sample_rate;
            let mut value: f32 = 0.0;
            for (j, &bit) in bits_group.iter().enumerate() {
                value += if bit == 1 {
                    (phase * multiplex_frequency[j]).sin() * 1.0
                } else {
                    -(phase * multiplex_frequency[j]).sin() * 1.0
                }
            }
            rtn.push(value);
        }
    }
    rtn
}

pub fn generate_frame_sample_from_bitvec(
    data: &BitStore,
    header: &Vec<f32>,
    multiplex_frequency: &[f32],
    sample_rate: u32,
    speed: u32,
) -> Vec<f32> {
    assert!(!multiplex_frequency.is_empty());
    let samples_per_bit: f32 = (sample_rate / speed) as f32;
    let mut rtn: Vec<f32> = header.clone();
    let sample_rate: f32 = sample_rate as f32;
    for (i, bits_group) in data.chunks(multiplex_frequency.len()).enumerate() {
        for time in i * (samples_per_bit as usize)..(i + 1) * (samples_per_bit as usize) {
            let phase: f32 = 2.0 * std::f32::consts::PI * time as f32 / sample_rate;
            let mut value: f32 = 0.0;
            for (j, bit) in bits_group.iter().enumerate() {
                value += if *bit {
                    (phase * multiplex_frequency[j]).sin() * 1.0
                } else {
                    -(phase * multiplex_frequency[j]).sin() * 1.0
                }
            }
            rtn.push(value);
        }
    }
    rtn
}
