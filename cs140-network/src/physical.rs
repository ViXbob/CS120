use crate::encoding::{BitStore, HandlePackage, NetworkPackage};
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::device::{InputDevice, OutputDevice};
use itertools::Itertools;
use std::collections::VecDeque;
use std::sync::Arc;
use std::thread::current;

pub struct PhysicalPackage(pub BitStore);

impl NetworkPackage for PhysicalPackage {}

type DefaultBuffer = RingBuffer<f32, 100000, false>;

pub struct PhysicalLayer {
    buffer: Arc<DefaultBuffer>,
    input: InputDevice<DefaultBuffer>,
    output: OutputDevice<DefaultBuffer>,
}

impl PhysicalLayer {
    fn create_header(
        header_length: usize,
        min_frequency: f32,
        max_frequency: f32,
        sample_rate: u32,
        scale: f32,
    ) -> Vec<f32> {
        let mut phase = 0.0;
        let time_gap = 1.0 / sample_rate as f32;

        let half = (header_length + 1) / 2;
        let half = (0..half).map(|x| {
            (max_frequency - min_frequency) / (half - 1) as f32 * x as f32 + min_frequency
        });
        let all = half
            .clone()
            .chain(half.rev().skip(if header_length % 2 == 0 { 0 } else { 1 }));

        all.map(|current_frequency| {
            phase += 2.0 * std::f32::consts::PI * time_gap * current_frequency;
            phase.sin() * scale
        })
        .collect::<Vec<f32>>()
    }

    fn header_detect(data: &[f32], header_length: usize, header: &[f32]) -> Option<usize> {
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

            println!("{}", now_correlation);

            if index == 2300 {
                println!("{}", now_correlation);
            }

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
}

impl HandlePackage<PhysicalPackage> for PhysicalLayer {
    fn send(&mut self, package: PhysicalPackage) {
        todo!()
    }

    fn receive(&mut self) -> PhysicalPackage {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::Arc;

    fn new() -> PhysicalLayer {
        let buffer = Arc::new(DefaultBuffer::new());
        let (input, _) = InputDevice::new(buffer.clone());
        let (output, _) = OutputDevice::new(buffer.clone());

        PhysicalLayer {
            buffer,
            input,
            output,
        }
    }

    #[test]
    fn test_play_header() {
        let layer = new();
        let result = PhysicalLayer::create_header(50000, 2000.0, 10000.0, 48000, 1.0);
        layer.buffer.push_by_ref(&result);
        let stop_play = layer.output.play();
        layer
            .buffer
            .push_by_iterator(48000, &mut std::iter::repeat(0.0f32));
        std::thread::sleep(std::time::Duration::from_secs(2));
        stop_play();
    }

    #[test]
    fn test_detect_header() {
        let result = PhysicalLayer::create_header(2400, 2000.0, 10000.0, 48000, 1.0);
        // let buffer_output = Arc::new(DefaultBuffer::new());
        // let buffer_input =  Arc::new(DefaultBuffer::new());
        // let (output,_) = OutputDevice::new(buffer_output.clone());
        // let (input,_) = InputDevice::new(buffer_input.clone());
        // let close_input = input.listen();
        // let close_output = output.play();
        // buffer_output.push_by_ref(&result);
        // std::thread::sleep(std::time::Duration::from_secs(1));
        // let data:Vec<f32> = buffer_input.pop_by_iterator(40000,|iter|iter.map(|x|*x).collect());

        let match_result = PhysicalLayer::header_detect(&result, result.len(), &result);
        println!("{}", 1);
    }
}
