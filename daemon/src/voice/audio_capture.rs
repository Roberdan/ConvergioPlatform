use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream};
use tokio::sync::mpsc;

use super::types::{AudioFrame, VoiceError};

/// Configuration for audio capture.
#[derive(Debug, Clone)]
pub struct CaptureConfig {
    /// Target sample rate (always resampled to this). Default: 16000.
    pub sample_rate: u32,
    /// Number of channels to capture. Default: 1 (mono).
    pub channels: u16,
    /// Frame duration in milliseconds. Default: 10ms.
    pub frame_duration_ms: u32,
}

impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            sample_rate: 16000,
            channels: 1,
            frame_duration_ms: 10,
        }
    }
}

impl CaptureConfig {
    /// Number of samples per frame: sample_rate * frame_duration_ms / 1000.
    pub fn frame_size(&self) -> usize {
        (self.sample_rate as usize * self.frame_duration_ms as usize) / 1000
    }
}

/// Events emitted by the capture stream (non-audio).
#[derive(Debug, Clone)]
pub enum CaptureEvent {
    Error(String),
    DeviceChanged,
}

impl std::fmt::Display for CaptureEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Error(msg) => write!(f, "capture error: {msg}"),
            Self::DeviceChanged => write!(f, "device changed"),
        }
    }
}

/// Rust-native microphone capture using cpal.
/// Produces `AudioFrame`s at 16kHz mono via a channel.
pub struct AudioCapture {
    config: CaptureConfig,
    running: Arc<AtomicBool>,
    // Hold the stream so it isn't dropped (which stops capture).
    _stream: Option<Stream>,
}

impl AudioCapture {
    pub fn new(config: CaptureConfig) -> Self {
        Self {
            config,
            running: Arc::new(AtomicBool::new(false)),
            _stream: None,
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn config(&self) -> &CaptureConfig {
        &self.config
    }

    /// Start capturing audio from the default input device.
    /// Returns a receiver that yields `AudioFrame`s at the configured rate.
    pub fn start(&mut self) -> Result<mpsc::UnboundedReceiver<AudioFrame>, VoiceError> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| {
            VoiceError::AudioError("no default audio input device".to_string())
        })?;

        let supported = device.default_input_config().map_err(|e| {
            VoiceError::AudioError(format!("device config error: {e}"))
        })?;

        let device_rate = supported.sample_rate().0;
        let device_channels = supported.channels();
        let sample_format = supported.sample_format();

        let (tx, rx) = mpsc::unbounded_channel::<AudioFrame>();
        let target_rate = self.config.sample_rate;
        let frame_size = self.config.frame_size();
        let running = self.running.clone();

        // Accumulate samples into frames before sending.
        let buffer = Arc::new(std::sync::Mutex::new(FrameBuffer::new(
            frame_size,
            target_rate,
        )));

        let stream_config: cpal::StreamConfig = supported.into();

        let buf_clone = buffer.clone();
        let tx_clone = tx.clone();
        let running_clone = running.clone();

        let stream = match sample_format {
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                &stream_config,
                device_rate,
                device_channels,
                target_rate,
                buf_clone,
                tx_clone,
                running_clone,
            )?,
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                &stream_config,
                device_rate,
                device_channels,
                target_rate,
                buf_clone,
                tx_clone,
                running_clone,
            )?,
            _ => {
                return Err(VoiceError::AudioError(format!(
                    "unsupported sample format: {sample_format:?}"
                )));
            }
        };

        stream.play().map_err(|e| {
            VoiceError::AudioError(format!("stream play error: {e}"))
        })?;

        self.running.store(true, Ordering::Relaxed);
        self._stream = Some(stream);
        Ok(rx)
    }

    /// Stop capturing audio.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self._stream = None;
    }
}

/// Internal buffer that accumulates samples and emits complete frames.
struct FrameBuffer {
    samples: Vec<i16>,
    frame_size: usize,
    target_rate: u32,
    frame_count: u64,
}

impl FrameBuffer {
    fn new(frame_size: usize, target_rate: u32) -> Self {
        Self {
            samples: Vec::with_capacity(frame_size * 2),
            frame_size,
            target_rate,
            frame_count: 0,
        }
    }

    /// Push samples and drain complete frames via the sender.
    fn push(&mut self, mono_16k: &[i16], tx: &mpsc::UnboundedSender<AudioFrame>) {
        self.samples.extend_from_slice(mono_16k);
        while self.samples.len() >= self.frame_size {
            let rest = self.samples.split_off(self.frame_size);
            let frame_samples = std::mem::replace(&mut self.samples, rest);
            let timestamp_ms =
                (self.frame_count * self.frame_size as u64 * 1000) / self.target_rate as u64;
            let frame = AudioFrame {
                samples: frame_samples,
                sample_rate: self.target_rate,
                timestamp_ms,
            };
            // If receiver dropped, silently stop — pipeline is shutting down.
            let _ = tx.send(frame);
            self.frame_count += 1;
        }
    }
}

/// Build a cpal input stream for a given sample type.
fn build_stream<T: cpal::Sample + cpal::SizedSample + Send + 'static>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    device_rate: u32,
    device_channels: u16,
    target_rate: u32,
    buffer: Arc<std::sync::Mutex<FrameBuffer>>,
    tx: mpsc::UnboundedSender<AudioFrame>,
    running: Arc<AtomicBool>,
) -> Result<Stream, VoiceError>
where
    i16: cpal::FromSample<T>,
{
    let stream = device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                if !running.load(Ordering::Relaxed) {
                    return;
                }
                // Convert to i16 via cpal's Sample trait.
                let i16_samples: Vec<i16> =
                    data.iter().map(|s| <i16 as cpal::FromSample<T>>::from_sample_(*s)).collect();

                // Stereo → mono if needed.
                let mono = if device_channels > 1 {
                    stereo_to_mono(&i16_samples)
                } else {
                    i16_samples
                };

                // Resample to target rate if needed.
                let resampled = resample(&mono, device_rate, target_rate);

                if let Ok(mut buf) = buffer.lock() {
                    buf.push(&resampled, &tx);
                }
            },
            |err| {
                eprintln!("audio capture error: {err}");
            },
            None,
        )
        .map_err(|e| VoiceError::AudioError(format!("build stream error: {e}")))?;
    Ok(stream)
}

/// Convert interleaved stereo samples to mono by averaging pairs.
pub fn stereo_to_mono(stereo: &[i16]) -> Vec<i16> {
    stereo
        .chunks_exact(2)
        .map(|pair| ((pair[0] as i32 + pair[1] as i32) / 2) as i16)
        .collect()
}

/// Linear resample from source_rate to target_rate.
/// Uses simple linear interpolation — good enough for speech at small ratios.
pub fn resample(samples: &[i16], source_rate: u32, target_rate: u32) -> Vec<i16> {
    if source_rate == target_rate || samples.is_empty() {
        return samples.to_vec();
    }
    let ratio = source_rate as f64 / target_rate as f64;
    let out_len = (samples.len() as f64 / ratio) as usize;
    (0..out_len)
        .map(|i| {
            let src_pos = i as f64 * ratio;
            let idx = src_pos as usize;
            let frac = src_pos - idx as f64;
            if idx + 1 < samples.len() {
                let a = samples[idx] as f64;
                let b = samples[idx + 1] as f64;
                (a + frac * (b - a)) as i16
            } else {
                samples[idx.min(samples.len() - 1)]
            }
        })
        .collect()
}
