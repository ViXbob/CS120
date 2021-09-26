use crate::buffer::Buffer as Buf;
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, StreamConfig, StreamError};

pub struct InputDevice<Buffer: Buf<f32>> {
    pub stream_config: (Device, StreamConfig, SampleFormat),
    /// store the audio data from the microphone, the data is packed per sampling
    pub audio_buffer: Arc<Buffer>,
}

impl<Buffer> InputDevice<Buffer>
where
    Buffer: Buf<f32> + 'static,
{
    /// new returns InputDevice as well as some config about the device / stream
    pub fn new(audio_buffer: Arc<Buffer>) -> (Self, (Device, StreamConfig, SampleFormat)) {
        (InputDevice {
            stream_config: Self::init_stream_config(),
            audio_buffer,
        }, Self::init_stream_config())
    }

    fn init_stream_config() -> (Device, StreamConfig, SampleFormat) {
        // Get the input device from user
        let host = cpal::default_host();
        let input_device = host
            .default_input_device()
            .expect("no input device available");
        // Choose the device that has the maximum of sample rates
        let config = input_device
            .default_input_config()
            .expect("error while querying configs");
        let sample_format = config.sample_format();
        (input_device, config.into(), sample_format)
    }

    pub fn listen(self) {
        let stream_config = self.stream_config.1.clone();
        let channels = stream_config.channels;
        let device = self.stream_config.0;
        let audio_buffer = self.audio_buffer;
        // Build the stream
        let stream = match self.stream_config.2 {
            SampleFormat::I16 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[i16], _: &_| {
                        Self::listen_handler(data, channels as usize, audio_buffer.clone())
                    },
                    Self::listen_error_handler,
                )
                .unwrap(),
            SampleFormat::U16 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[u16], _: &_| {
                        Self::listen_handler(data, channels as usize, audio_buffer.clone())
                    },
                    Self::listen_error_handler,
                )
                .unwrap(),
            SampleFormat::F32 => device
                .build_input_stream(
                    &stream_config,
                    move |data: &[f32], _: &_| {
                        Self::listen_handler(data, channels as usize, audio_buffer.clone())
                    },
                    Self::listen_error_handler,
                )
                .unwrap(),
        };
        stream.play().unwrap();
        std::thread::park();
    }

    fn listen_handler<T>(input: &[T], channels: usize, audio_buffer: Arc<Buffer>)
    where
        T: cpal::Sample,
    {
        let mut iterator = input.iter().step_by(channels).map(|value| value.to_f32());
        audio_buffer.push_by_iterator(input.len() / channels, &mut iterator);
    }

    fn listen_error_handler(err: StreamError) {
        panic!("{}", err)
    }
}

pub struct OutputDevice<Buffer: Buf<f32>> {
    stream_config: (Device, StreamConfig, SampleFormat),
    /// play the audio from audio buffer, consumes n packed data per play, where n is the number of channels to play
    audio_buffer: Arc<Buffer>,
}

impl<Buffer> OutputDevice<Buffer>
where
    Buffer: Buf<f32> + 'static,
{
    /// new returns InputDevice as well as some config about the device / stream, for example: channels
    pub fn new(audio_buffer: Arc<Buffer>) -> Self {
        OutputDevice {
            stream_config: Self::init_stream_config(),
            audio_buffer,
        }
    }

    fn init_stream_config() -> (Device, StreamConfig, SampleFormat) {
        // Get the input device from user
        let host = cpal::default_host();
        let output_device = host
            .default_output_device()
            .expect("no input device available");
        // Choose the device that has the maximum of sample rates
        let config = output_device
            .default_output_config()
            .expect("error while querying configs");
        let sample_format = config.sample_format();
        (output_device, config.into(), sample_format)
    }

    pub fn play(self) {
        let stream_config = self.stream_config.1.clone();
        let device = self.stream_config.0;
        let audio_buffer = self.audio_buffer;
        let channels = stream_config.channels as usize;
        // Build the stream
        let stream = match self.stream_config.2 {
            SampleFormat::I16 => device
                .build_output_stream(
                    &stream_config,
                    move |data: &mut [i16], _: &_| {
                        Self::play_handler(data, channels, audio_buffer.clone())
                    },
                    Self::play_error_handler,
                )
                .unwrap(),
            SampleFormat::U16 => device
                .build_output_stream(
                    &stream_config,
                    move |data: &mut [u16], _: &_| {
                        Self::play_handler(data, channels, audio_buffer.clone())
                    },
                    Self::play_error_handler,
                )
                .unwrap(),
            SampleFormat::F32 => device
                .build_output_stream(
                    &stream_config,
                    move |data: &mut [f32], _: &_| {
                        Self::play_handler(data, channels, audio_buffer.clone())
                    },
                    Self::play_error_handler,
                )
                .unwrap(),
        };
        stream.play().unwrap();
        std::thread::park();
    }

    fn play_handler<T>(output: &mut [T], channels: usize, audio_buffer: Arc<Buffer>)
    where
        T: cpal::Sample,
    {
        audio_buffer.pop(output.len() / channels, move |first, second| {
            for (frame, value) in output
                .chunks_mut(channels)
                .zip(first.iter().chain(second.iter()))
            {
                for sample in frame.iter_mut() {
                    *sample = cpal::Sample::from(value);
                }
            }
        });
    }

    fn play_error_handler(err: StreamError) {
        panic!("{}", err)
    }
}
