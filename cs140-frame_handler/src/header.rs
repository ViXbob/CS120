use nalgebra::*;
use rustfft::{num_complex::Complex, FftPlanner};
use std::collections::VecDeque;

pub fn header_solve(
    count: usize,
    data: &[f32],
    frequency: u32,
    sample_rate: u32,
) -> Result<Option<usize>, &'static str> {
    let sample_rate = sample_rate as f32;
    let frequency = frequency as f32;
    let data: Vec<f32> = data.iter().copied().collect();
    let period_count = (sample_rate / frequency) as usize * 2;
    if count < period_count / 2 {
        return Err("data size is less than samples in one header");
    }
    let coefficient: f32 = frequency * 2.0 * std::f32::consts::PI;
    let matrix_a = DMatrix::from_fn(period_count, 2, |r, c| -> f32 {
        let t: f32 = coefficient * r as f32 / sample_rate;
        if c == 0 {
            t.sin()
        } else {
            t.cos()
        }
    });
    let matrix_b = DMatrix::from_fn(period_count, 1, |r, _c| data[r]);
    let x = (matrix_a.transpose() * matrix_a.clone())
        .try_inverse()
        .unwrap()
        * matrix_a.transpose()
        * matrix_b;
    let cos_phase = x[(0, 0)];
    let sin_phase = x[(1, 0)];
    let phase = (sin_phase / cos_phase).atan();
    let amplitude = (sin_phase * sin_phase + cos_phase * cos_phase).sqrt();
    for (index, &value) in data.iter().enumerate() {
        if (value - amplitude * (index as f32 * coefficient / sample_rate + phase).sin()).abs()
            > 1e-5
        {
            return Ok(Some(index));
        }
    }
    Err("the frame has only header")
}

pub fn header_detect(data: &[f32], header_length: usize, header: &[f32]) -> Option<usize> {
    let correlation_value = |data_slice: &VecDeque<f32>| {
        let mut rtn: f32 = 0.0;
        for (x, y) in data_slice.iter().zip(header.iter()) {
            rtn += x * y;
        }
        rtn
    };

    let sum = header.iter().map(|x: _| x * x).sum::<f32>();

    let mut sync: VecDeque<f32> =
        VecDeque::from((0..header_length).map(|_| 0.0).collect::<Vec<f32>>());
    let mut power: f32 = 0.0;
    let mut start_index: usize = 0;
    let mut max_correlation: f32 = 0.0;

    // let mut max_ratio = 0;

    for (index, &value) in data.iter().enumerate() {
        // if(index + )
        power = (power * 63.0 + value * value) / 64.0;
        sync.pop_front();
        sync.push_back(value);
        let now_correlation: f32 = correlation_value(&sync) / sum;
        // if index > 2300 && index < 2600 {
        // println!("{}", now_correlation);
        // }
        // max_correlation = std::max(max_correlation, now_correlation);
        // if now_correlation > max_correlation {
        //     max_correlation = now_correlation;
        //     println!("{}, {}", power, now_correlation);
        // }

        if now_correlation > power * 1.5
            && (now_correlation > max_correlation
                || ((now_correlation - max_correlation).abs() < 1e-7 && index > start_index))
            && now_correlation > 0.1
        {
            max_correlation = now_correlation;
            start_index = index;
        } else if (index - start_index > 200) && start_index != 0 {
            return Some(start_index + 1);
        }
    }
    println!("{}", max_correlation);
    None
}

pub fn frame_resolve(
    data: &[f32],
    header_length: usize,
    header: &[f32],
    multiplex_range: usize,
    multiplex_frequency: &[f32],
    sample_rate: u32,
    speed: u32,
    frame_length: usize,
) -> Result<(Vec<i32>, usize), &'static str> {
    let begin_index = header_detect(data, header_length, header).expect("detection failed");
    let sample_per_bit = sample_rate / speed;
    let fft_len: usize = sample_per_bit as usize;
    let mut planner = FftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(fft_len);
    let mut buffer: Vec<Complex<f32>> = Vec::new();
    let mut result: Vec<i32> = Vec::new();
    for i in 0..frame_length {
        buffer = data[(begin_index + i * sample_per_bit as usize)
            ..(begin_index + (i + 1) * sample_per_bit as usize)]
            .iter()
            .map(|x: _| Complex::<f32>::new(*x, 0.0))
            .collect();
        fft.process(buffer.as_mut_slice());
        for frequency in multiplex_frequency {
            let index: usize = (*frequency as usize) / ((sample_rate / sample_per_bit) as usize);
            let value = buffer[index];
            println!("{}", value.im / (sample_per_bit as f32) * 2.0);
            if (value.im.abs() / (sample_per_bit as f32) * 2.0 > 0.8) && (value.im < 0.0) {
                result.push(1);
            } else if (value.im.abs() / (sample_per_bit as f32) * 2.0 > 0.8) && (value.im > 0.0) {
                result.push(0);
            } else {
                return Err("bit lost");
            }
        }
    }
    Ok((
        result,
        (begin_index + frame_length * sample_per_bit as usize) as usize,
    ))
}

pub fn header_create(
    header_length: usize,
    min_frequency: f32,
    max_frequency: f32,
    sample_rate: u32,
    scale: f32,
) -> Vec<f32> {
    let mut phase: f32 = 0.0;
    let mut cur_frequency: f32 = min_frequency;
    let frequency_step = (max_frequency - min_frequency) / (header_length as f32 / 2.0);
    let time_gap: f32 = 1.0 / sample_rate as f32;

    (0..header_length)
        .map(|x: _| {
            cur_frequency += if x * 2 < header_length {
                frequency_step
            } else {
                -frequency_step
            };
            phase += 2.0 * std::f32::consts::PI * time_gap * cur_frequency;
            phase.sin() * scale
        })
        .collect::<Vec<f32>>()
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
    let mut rtn: Vec<f32> = header_create(220, 3000.0, 6000.0, sample_rate, 1.0);
    // for i in 1..20 {
    //     rtn.extend(header_create(220, 3000.0, 6000.0, sample_rate, 1.0));
    // }
    let sample_rate: f32 = sample_rate as f32;
    for (i, bits_group) in data.chunks(multiplex_range).enumerate() {
        for time in i * (samples_per_bit as usize)..(i + 1) * (samples_per_bit as usize) {
            let phase: f32 = 2.0 * std::f32::consts::PI * time as f32 / sample_rate;
            let mut value: f32 = 0.0;
            for (j, &bit) in bits_group.iter().enumerate() {
                value += if bit == 1 {
                    (phase * multiplex_frequency[j]).sin()
                } else {
                    -(phase * multiplex_frequency[j]).sin()
                }
            }
            rtn.push(value);
        }
    }
    rtn
}
