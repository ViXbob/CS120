fn main() {
    let initial_phase: f32 = 0_f32;
    let sample_rate: u32 = 48000;
    let frequency: u32 = 777;
    let period: u32 = sample_rate / frequency as u32;
    let mut data: Vec<f32> = (0..period * 5 + 1)
        .map(|x| {
            (2.0 * std::f32::consts::PI * x as f32 / sample_rate as f32 * frequency as f32
                + initial_phase)
                .sin()
        })
        .collect();
    data.push(0.77777);
    let index = cs140_frame_handler::header::header_solve(
        data.len(),
        data.as_slice(),
        frequency,
        sample_rate,
    )
    .unwrap()
    .unwrap();
    println!("{}", index);
}
