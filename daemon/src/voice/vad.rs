use super::types::{AudioFrame, SpeechSegment, VoiceError};

/// Voice Activity Detection using energy-based detection.
/// Detects speech onset within ~50ms (800 samples at 16kHz).
pub struct VoiceActivityDetector {
    threshold: f32,
    min_speech_ms: u64,
    min_silence_ms: u64,
    in_speech: bool,
    speech_start_ms: u64,
    silence_start_ms: u64,
    current_samples: Vec<i16>,
}

impl VoiceActivityDetector {
    pub fn new(threshold: f32) -> Self {
        Self {
            threshold: threshold.clamp(0.0, 1.0),
            min_speech_ms: 100,
            min_silence_ms: 300,
            in_speech: false,
            speech_start_ms: 0,
            silence_start_ms: 0,
            current_samples: Vec::new(),
        }
    }

    /// Process an audio frame. Returns a SpeechSegment when speech ends.
    pub fn process(&mut self, frame: &AudioFrame) -> Result<Option<SpeechSegment>, VoiceError> {
        let energy = compute_energy(&frame.samples);
        let is_voice = energy > self.threshold;

        if is_voice {
            if !self.in_speech {
                self.in_speech = true;
                self.speech_start_ms = frame.timestamp_ms;
                self.current_samples.clear();
            }
            self.current_samples.extend_from_slice(&frame.samples);
            self.silence_start_ms = 0;
            Ok(None)
        } else if self.in_speech {
            if self.silence_start_ms == 0 {
                self.silence_start_ms = frame.timestamp_ms;
            }
            self.current_samples.extend_from_slice(&frame.samples);
            let silence_duration = frame.timestamp_ms.saturating_sub(self.silence_start_ms);
            let speech_duration = frame.timestamp_ms.saturating_sub(self.speech_start_ms);

            if silence_duration >= self.min_silence_ms && speech_duration >= self.min_speech_ms {
                self.in_speech = false;
                let segment = SpeechSegment {
                    start_ms: self.speech_start_ms,
                    end_ms: frame.timestamp_ms,
                    samples: std::mem::take(&mut self.current_samples),
                };
                Ok(Some(segment))
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    }

    /// Reset the detector state.
    pub fn reset(&mut self) {
        self.in_speech = false;
        self.speech_start_ms = 0;
        self.silence_start_ms = 0;
        self.current_samples.clear();
    }

    pub fn is_in_speech(&self) -> bool {
        self.in_speech
    }
}

/// Compute RMS energy of audio samples, normalized to [0.0, 1.0].
fn compute_energy(samples: &[i16]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();
    let rms = (sum / samples.len() as f64).sqrt();
    (rms / i16::MAX as f64) as f32
}
