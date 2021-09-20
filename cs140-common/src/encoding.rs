use crate::audio_buffer::AudioBuffer;
use std::sync::Arc;

pub struct AudioEncoder<Buffer: AudioBuffer> {
    audio_buffer: Arc<Buffer>,
}

impl<Buffer> AudioEncoder<Buffer>
where
    Buffer: AudioBuffer,
{
    pub fn new(audio_buffer: Arc<Buffer>) -> Self {
        AudioEncoder { audio_buffer }
    }

    /// Encode binary data into the audio, and put the data into audio storage, returning a key that can be used to access it.
    /// This function will be blocked until there is space in audio storage.
    ///
    /// We will compute the data based on the provided data, so we receive the reference
    pub fn encode<T>(&self, raw_data: &T, encoder: impl Fn(&T) -> Buffer::Data) {
        self.audio_buffer.push(encoder(raw_data))
    }
}

pub struct AudioDecoder<Buffer: AudioBuffer> {
    audio_storage: Arc<Buffer>,
}

impl<Buffer> AudioDecoder<Buffer>
where
    Buffer: AudioBuffer,
{
    pub fn new(audio_storage: Arc<Buffer>) -> Self {
        AudioDecoder { audio_storage }
    }

    /// Decode binary data in the audio storage indexed by the provided key, returning the data decoded by decoder.
    pub fn decode<T>(&self, decoder: impl Fn(&Buffer::Data) -> T) -> T {
        self.audio_storage.pop(decoder)
    }
}
