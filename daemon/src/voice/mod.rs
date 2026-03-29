#[cfg(feature = "voice")]
pub mod audio_capture;
pub mod audio_util;
pub mod intent;
#[cfg(feature = "voice")]
pub mod pipeline;
pub mod types;
#[cfg(feature = "voice")]
pub mod vad;
pub mod wake_word;
pub mod whisper;

#[cfg(feature = "voice")]
pub use audio_capture::{AudioCapture, CaptureConfig};
#[cfg(feature = "voice")]
pub use pipeline::VoicePipeline;
pub use types::{VoiceConfig, VoiceError, VoiceState};

#[cfg(all(test, feature = "voice"))]
#[path = "audio_capture_tests.rs"]
mod audio_capture_tests;

#[cfg(all(test, feature = "voice"))]
#[path = "vad_tests.rs"]
mod vad_tests;

#[cfg(test)]
#[path = "wake_word_tests.rs"]
mod wake_word_tests;

#[cfg(test)]
#[path = "intent_tests.rs"]
mod intent_tests;

#[cfg(all(test, feature = "voice"))]
#[path = "pipeline_tests.rs"]
mod pipeline_tests;

#[cfg(test)]
#[path = "whisper_tests.rs"]
mod whisper_tests;

#[cfg(all(test, feature = "voice"))]
#[path = "integration_tests.rs"]
mod integration_tests;
