use nalgebra::*;
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

    for (index, &value) in data.iter().enumerate() {
        power = (power * 63.0 + value * value) / 64.0;
        sync.pop_front();
        sync.push_back(value);
        let now_correlation: f32 = correlation_value(&sync) / sum;

        println!("{}", now_correlation);

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