use std::collections::VecDeque;
use log::trace;

/// Detect the position of the start of data from audio using `header`
///
/// If `None` returned, it means we do not find header from the audio
pub fn detect_header<'a>(
    data_iter: impl Iterator<Item = &'a f32>,
    header: &[f32],
) -> Option<usize> {
    let header_length = header.len();
    let correlation_value = |data_slice: &VecDeque<f32>| {
        data_slice
            .iter()
            .zip(header.iter())
            .fold(0.0, |old, (x, y)| old + x * y)
    };
    let sum = header.iter().fold(0.0, |old, x| old + x * x);
    let mut sync: VecDeque<f32> = VecDeque::from(
        std::iter::repeat(0.0)
            .take(header_length)
            .collect::<Vec<f32>>(),
    );
    let mut power: f32 = 0.0;
    let mut start_index: usize = 0;
    let mut max_correlation: f32 = 0.0;

    for (index, &value) in data_iter.enumerate() {
        power = (power * (header_length - 1) as f32 + value * value) / (header_length as f32);
        sync.pop_front();
        sync.push_back(value);
        let now_correlation: f32 = correlation_value(&sync) / sum;
        // if now_correlation > power * 2.0
        //     && (now_correlation > max_correlation
        //         || ((now_correlation - max_correlation).abs() < 1e-7 && index > start_index))
        //     && now_correlation > 0.15
        // println!("update: {}, {}, {}, {}", sum, now_correlation, max_correlation, power);
        if now_correlation > power
            && now_correlation > max_correlation
            && now_correlation > 0.5
        {
            trace!("update: {}, {}, {}, {}", sum, now_correlation, max_correlation, power);
            max_correlation = now_correlation;
            start_index = index;
        } else if (index - start_index > header_length) && start_index != 0 {
            // println!("detect: {}, {}, {}, {}, {}", sum, now_correlation, max_correlation, power, value);
            return Some(start_index + 1);
        }
    }
    None
}

/// Create a header by given parameters.
pub fn create_header(
    header_length: usize,
    min_frequency: f32,
    max_frequency: f32,
    sample_rate: u32,
) -> Vec<f32> {
    let scale = 1.0;
    let mut phase = 0.0;
    let time_gap = 1.0 / sample_rate as f32;

    let half = (header_length + 1) / 2;
    let half = (0..half)
        .map(|x| (max_frequency - min_frequency) / (half - 1) as f32 * x as f32 + min_frequency);
    let all = half
        .clone()
        .chain(half.rev().skip(if header_length % 2 == 0 { 0 } else { 1 }));

    all.map(|current_frequency| {
        phase += 2.0 * std::f32::consts::PI * time_gap * current_frequency;
        phase.sin() * scale
    })
    .collect::<Vec<f32>>()
}

#[cfg(test)]
mod test {
    use super::*;
    use rand::Rng;
    #[test]
    fn test_detect_header() {
        let pos = 2200;
        let header = create_header(200, 3000.0, 7000.0, 48000);
        let data = (0..12000)
            .map(|index| {
                if index >= pos && index < header.len() + pos {
                    (rand::thread_rng()
                        .gen_range(-std::f32::consts::PI..std::f32::consts::PI)
                        .sin()
                        * 0.5
                        + header[index - pos])
                        / 2.0
                } else {
                    rand::thread_rng()
                        .gen_range(-std::f32::consts::PI..std::f32::consts::PI)
                        .sin()
                        * 0.5
                }
            })
            .collect::<Vec<f32>>();
        let header_position_guessed = detect_header(data.iter(), &header);
        println!("{}", header_position_guessed.unwrap());
    }
}
