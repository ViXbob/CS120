use cpal::{Device, SampleFormat, StreamConfig};
use cs140_buffer::ring_buffer::RingBuffer;
use cs140_common::buffer::Buffer;
use cs140_common::device::InputDevice;
use super::sample_format;
use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};

pub fn record(path : &str, record_time : u32) -> Result<(), anyhow::Error> {
    let buffer: RingBuffer<f32, 100001, false> = RingBuffer::new();
    let buffer_ptr = Arc::new(buffer);
    let (input, input_config) = InputDevice::new(buffer_ptr.clone());
    let input_close = input.listen();

    let spec = wav_spec_from_config(&input_config);
    // println!("{:?}", spec);

    let writer = hound::WavWriter::create(path, spec)?;
    let writer = Arc::new(Mutex::new(Some(writer)));

    let segment_count = 100;
    let segment_len = input_config.0.sample_rate.0 / segment_count;

    for _ in 0..record_time * segment_count {
        buffer_ptr.pop_by_ref(segment_len as usize, |data| {
            write_input_data::<f32, f32>(data, &writer)
        });
    }
    writer.lock().unwrap().take().unwrap().finalize()?;

    input_close();
    Ok(())
}

fn wav_spec_from_config(input: &(StreamConfig, SampleFormat)) -> hound::WavSpec {
    hound::WavSpec {
        channels: input.0.channels as _,
        sample_rate: input.0.sample_rate.0 as _,
        bits_per_sample: (input.1.sample_size() * 8) as _,
        sample_format: sample_format(input.1),
    }
}

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn write_input_data<T, U>(input: &[T], writer: &WavWriterHandle)
    where
        T: cpal::Sample,
        U: cpal::Sample + hound::Sample,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for &sample in input.iter() {
                let sample: U = cpal::Sample::from(&sample);
                for _i in 0..writer.spec().channels {
                    writer.write_sample(sample).ok();
                }
            }
        }
    }
}
