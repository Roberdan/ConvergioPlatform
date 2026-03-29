pub mod intent;
pub mod pipeline;
pub mod types;
pub mod vad;
pub mod wake_word;
pub mod whisper;

pub use pipeline::VoicePipeline;
pub use types::{VoiceConfig, VoiceError, VoiceState};

#[cfg(test)]
#[path = "vad_tests.rs"]
mod vad_tests;

#[cfg(test)]
#[path = "wake_word_tests.rs"]
mod wake_word_tests;

#[cfg(test)]
#[path = "intent_tests.rs"]
mod intent_tests;

#[cfg(test)]
#[path = "pipeline_tests.rs"]
mod pipeline_tests;
