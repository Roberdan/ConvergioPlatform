use super::types::{AudioFrame, SpeechSegment, VoiceError};

/// Aggressiveness level returned by threshold mapping (testable without PartialEq on VadMode).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VadAggressiveness {
    Quality,
    LowBitrate,
    Aggressive,
    VeryAggressive,
}

impl VadAggressiveness {
    fn to_webrtc(self) -> webrtc_vad::VadMode {
        match self {
            Self::Quality => webrtc_vad::VadMode::Quality,
            Self::LowBitrate => webrtc_vad::VadMode::LowBitrate,
            Self::Aggressive => webrtc_vad::VadMode::Aggressive,
            Self::VeryAggressive => webrtc_vad::VadMode::VeryAggressive,
        }
    }
}

/// Map VAD threshold (0.0–1.0) to aggressiveness level.
/// Lower threshold = more permissive (Quality), higher = stricter (VeryAggressive).
pub fn threshold_to_vad_mode(threshold: f32) -> VadAggressiveness {
    let t = threshold.clamp(0.0, 1.0);
    if t < 0.25 {
        VadAggressiveness::Quality
    } else if t < 0.5 {
        VadAggressiveness::LowBitrate
    } else if t < 0.75 {
        VadAggressiveness::Aggressive
    } else {
        VadAggressiveness::VeryAggressive
    }
}

/// Voice Activity Detection backed by libwebrtc's VAD (webrtc-vad crate).
/// Detects speech onset within ~10ms frames at 16kHz.
pub struct VoiceActivityDetector {
    vad: webrtc_vad::Vad,
    min_speech_ms: u64,
    min_silence_ms: u64,
    in_speech: bool,
    speech_start_ms: u64,
    silence_start_ms: u64,
    current_samples: Vec<i16>,
}

impl VoiceActivityDetector {
    pub fn new(threshold: f32) -> Self {
        let aggressiveness = threshold_to_vad_mode(threshold);
        let mut vad = webrtc_vad::Vad::new_with_rate(webrtc_vad::SampleRate::Rate16kHz);
        vad.set_mode(aggressiveness.to_webrtc());
        Self {
            vad,
            min_speech_ms: 100,
            min_silence_ms: 300,
            in_speech: false,
            speech_start_ms: 0,
            silence_start_ms: 0,
            current_samples: Vec::new(),
        }
    }

    /// Process an audio frame. Returns a SpeechSegment when speech ends.
    /// Frame must be 10ms (160), 20ms (320), or 30ms (480) samples at 16kHz.
    pub fn process(&mut self, frame: &AudioFrame) -> Result<Option<SpeechSegment>, VoiceError> {
        let is_voice = self
            .vad
            .is_voice_segment(&frame.samples)
            .map_err(|_| VoiceError::VadError("invalid frame size for webrtc-vad".into()))?;

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

// SAFETY: webrtc_vad::Vad wraps a C fvad struct (single-owner, no shared state).
// The raw *mut Fvad prevents auto-Send, but the struct is only accessed via &mut self
// and each VoiceActivityDetector owns its fvad instance exclusively.
unsafe impl Send for VoiceActivityDetector {}
