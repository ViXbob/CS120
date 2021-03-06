use std::sync::Arc;
use std::thread;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, SampleRate, StreamConfig, StreamError, SupportedBufferSize};
use tokio::runtime::{Builder, Runtime};

use crate::buffer::Buffer as Buf;
use crate::descriptor::SoundDescriptor;
use crate::padding::padding_range;

pub struct InputDevice<Buffer: Buf<f32>> {
    stream_config: (Device, StreamConfig, SampleFormat),
    /// store the audio data from the microphone, the data is packed per sampling
    audio_buffer: Arc<Buffer>,
}

const SAMPLE_RATE: u32 = 48000;

impl<Buffer> InputDevice<Buffer>
    where
        Buffer: Buf<f32> + 'static,
{
    /// new returns InputDevice as well as some config about the device / stream
    pub fn new(audio_buffer: Arc<Buffer>) -> (Self, SoundDescriptor) {
        // let config = Self::init_stream_config(&"USB Audio Device");
        let config = Self::init_stream_config(0);
        let descriptor = SoundDescriptor {
            channels: config.1.channels,
            sample_rate: config.1.sample_rate.0,
            sample_format: config.2.into(),
        };
        (
            InputDevice {
                stream_config: config,
                audio_buffer,
            },
            descriptor,
        )
    }

    pub fn new_with_specific_device(audio_buffer: Arc<Buffer>, device_name: usize) -> (Self, SoundDescriptor) {
        let config = Self::init_stream_config(device_name);
        let descriptor = SoundDescriptor {
            channels: config.1.channels,
            sample_rate: config.1.sample_rate.0,
            sample_format: config.2.into(),
        };
        (
            InputDevice {
                stream_config: config,
                audio_buffer,
            },
            descriptor,
        )
    }

    fn init_stream_config(device_name: usize) -> (Device, StreamConfig, SampleFormat) {
        // Get the input device from user
        let host = cpal::default_host();
        // let host = if cfg!(target_os = "windows")
        // {
        //     cpal::host_from_id(cpal::HostId::Asio).expect("failed to initialise ASIO host")
        // } else {
        //     cpal::default_host()
        // };
        let choose_device = || {
            // for (index, input_) in host.input_devices().unwrap().enumerate() {
            //     println!("input_device {}: {}", index, input_.name().unwrap());
            // }
            for (index, input_) in host.input_devices().unwrap().enumerate() {
                if index == device_name {
                    return input_;
                }
            }
            return host.default_input_device().expect("no input device available");
        };
        // let input_device = host
        //     .default_input_device()
        //     .expect("no input device available");
        let input_device = choose_device();
        // Choose the device that has the maximum of sample rates
        println!("using {} as input device", input_device.name().unwrap());
        let mut config = input_device
            .default_input_config()
            .expect("error while querying configs");
        // Choose the device that has the maximum of sample rates
        for _config in input_device.supported_input_configs().unwrap() {
            // println!("{:?}", _config.max_sample_rate());
            // println!("{:?}", _config.buffer_size());
            // println!("{:?}", _config.channels());
            if _config.max_sample_rate().0 >= SAMPLE_RATE && _config.channels() == 2 {
                config = _config.with_sample_rate(SampleRate{0:SAMPLE_RATE});
                break;
            }
        }
        let config_ = config.clone();
        let sample_format = config.sample_format();
        let mut config: StreamConfig = config.into();
        println!("with sample rate:{:?}", config.sample_rate);
        let buffer_size = match config_.buffer_size() {
            SupportedBufferSize::Range { min, max } => {
                // cpal::BufferSize::Fixed(*min)
                config.buffer_size
            }
            SupportedBufferSize::Unknown => {
                config.buffer_size
            }
        };
        config.buffer_size = buffer_size;
        (input_device, config, sample_format)
    }

    pub fn listen(self) -> impl FnOnce() -> Self {
        let thread_handle = std::thread::spawn(move || {
            let stream_config = &self.stream_config.1;
            let channels = stream_config.channels;
            let device = &self.stream_config.0;
            let audio_buffer = self.audio_buffer.clone();
            let rt = Builder::new_multi_thread().enable_all().build().unwrap();
            // Build the stream
            let stream = match self.stream_config.2 {
                SampleFormat::I16 => device
                    .build_input_stream(
                        stream_config,
                        move |data: &[i16], _: &_| {
                            Self::listen_handler(data, channels as usize, audio_buffer.clone(), &rt);
                        },
                        Self::listen_error_handler,
                    )
                    .unwrap(),
                SampleFormat::U16 => device
                    .build_input_stream(
                        stream_config,
                        move |data: &[u16], _: &_| {
                            Self::listen_handler(data, channels as usize, audio_buffer.clone(), &rt);
                        },
                        Self::listen_error_handler,
                    )
                    .unwrap(),
                SampleFormat::F32 => device
                    .build_input_stream(
                        stream_config,
                        move |data: &[f32], _: &_| {
                            Self::listen_handler(data, channels as usize, audio_buffer.clone(), &rt);
                        },
                        Self::listen_error_handler,
                    )
                    .unwrap(),
            };
            stream.play().unwrap();
            thread::park();
            self
        });

        move || {
            thread_handle.thread().unpark();
            thread_handle.join().unwrap()
        }
    }

    fn listen_handler<T>(input: &[T], channels: usize, audio_buffer: Arc<Buffer>, rt: &Runtime)
        where
            T: cpal::Sample + Sync,
    {
        let mut iterator = input.iter().step_by(channels).map(|value| value.to_f32());
        rt.block_on(audio_buffer.push_by_iterator(input.len() / channels, &mut iterator));
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
    pub fn new(audio_buffer: Arc<Buffer>) -> (Self, SoundDescriptor) {
        // let config = Self::init_stream_config(&"USB Audio Device");
        let config = Self::init_stream_config(0);
        let descriptor = SoundDescriptor {
            channels: config.1.channels,
            sample_rate: config.1.sample_rate.0,
            sample_format: config.2.into(),
        };
        (
            OutputDevice {
                stream_config: config,
                audio_buffer,
            },
            descriptor,
        )
    }

    pub fn new_with_specific_device(audio_buffer: Arc<Buffer>, device_name: usize) -> (Self, SoundDescriptor) {
        let config = Self::init_stream_config(device_name);
        let descriptor = SoundDescriptor {
            channels: config.1.channels,
            sample_rate: config.1.sample_rate.0,
            sample_format: config.2.into(),
        };
        (
            OutputDevice {
                stream_config: config,
                audio_buffer,
            },
            descriptor,
        )
    }

    pub fn sound_descriptor(&self) -> SoundDescriptor {
        SoundDescriptor {
            channels: self.stream_config.1.channels,
            sample_rate: self.stream_config.1.sample_rate.0,
            sample_format: self.stream_config.2.into(),
        }
    }

    fn init_stream_config(device_name: usize) -> (Device, StreamConfig, SampleFormat) {
        // Get the input device from user
        let host = cpal::default_host();
        // let host = if cfg!(target_os = "windows")
        // {
        //     cpal::host_from_id(cpal::HostId::Asio).expect("failed to initialise ASIO host")
        // } else {
        //     cpal::default_host()
        // };
        let choose_device = || {
            // for (index, output_) in host.output_devices().unwrap().enumerate() {
            //     println!("output_device {}: {}", index, output_.name().unwrap());
            // }
            for (index, output_) in host.output_devices().unwrap().enumerate() {
                if index == device_name {
                    return output_;
                }
            }
            return host.default_output_device().expect("no output device available");
        };

        // let output_device = host
        //     .default_output_device()
        //     .expect("no output device available");
        let output_device = choose_device();
        println!("using {} as output device", output_device.name().unwrap());

        let mut config = output_device
            .default_output_config()
            .expect("error while querying configs");


        // Choose the device that has the maximum of sample rates
        for _config in output_device.supported_output_configs().unwrap() {
            // println!("{:?}", _config.max_sample_rate());
            // println!("{:?}", _config.buffer_size());
            // println!("{:?}", _config.channels());
            if _config.max_sample_rate().0 >= SAMPLE_RATE && _config.channels() == 2 {
                config = _config.with_sample_rate(SampleRate{0:SAMPLE_RATE});
                break;
            }
        }
        let config_ = config.clone();
        let sample_format = config.sample_format();
        let mut config: StreamConfig = config.into();
        let buffer_size = match config_.buffer_size() {
            SupportedBufferSize::Range { min, max } => {
                // cpal::BufferSize::Fixed(*min)
                config.buffer_size
            }
            SupportedBufferSize::Unknown => {
                config.buffer_size
            }
        };
        println!("with sample rate:{:?}", config.sample_rate);
        config.buffer_size = buffer_size;
        (output_device, config, sample_format)
    }

    pub fn play(self) -> impl FnOnce() -> Self {
        let thread_handle = std::thread::spawn(move || {
            let stream_config = &self.stream_config.1;
            let device = &self.stream_config.0;
            let audio_buffer = self.audio_buffer.clone();
            let channels = stream_config.channels as usize;

            // Build the stream
            let stream = match self.stream_config.2 {
                SampleFormat::I16 => device
                    .build_output_stream(
                        stream_config,
                        move |data: &mut [i16], _: &_| {
                            Self::play_handler(data, channels, audio_buffer.clone());
                        },
                        Self::play_error_handler,
                    )
                    .unwrap(),
                SampleFormat::U16 => device
                    .build_output_stream(
                        stream_config,
                        move |data: &mut [u16], _: &_| {
                            Self::play_handler(data, channels, audio_buffer.clone());
                        },
                        Self::play_error_handler,
                    )
                    .unwrap(),
                SampleFormat::F32 => device
                    .build_output_stream(
                        stream_config,
                        move |data: &mut [f32], _: &_| {
                            Self::play_handler(data, channels, audio_buffer.clone());
                        },
                        Self::play_error_handler,
                    )
                    .unwrap(),
            };

            stream.play().unwrap();
            thread::park();
            self
        });

        move || {
            thread_handle.thread().unpark();
            thread_handle.join().unwrap()
        }
    }

    fn play_handler<T>(output: &mut [T], channels: usize, audio_buffer: Arc<Buffer>)
        where
            T: cpal::Sample,
    {
        let len = output.len() / channels;
        audio_buffer.must_pop(len, move |first, second| {
            for (frame, value) in output
                .chunks_mut(channels)
                .zip(first.iter().chain(second.iter()))
            {
                for sample in frame.iter_mut() {
                    *sample = cpal::Sample::from(value);
                }
            }
            ((), len)
        }, padding_range(-0.0001, 0.0001));
    }

    fn play_error_handler(err: StreamError) {
        panic!("{}", err)
    }
}
