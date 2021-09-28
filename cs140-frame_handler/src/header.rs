use nalgebra::*;

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
    let x = (matrix_a.transpose() * matrix_a.clone()).try_inverse().unwrap() * matrix_a.transpose() * matrix_b;
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
