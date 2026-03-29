use tokio::sync::mpsc;

use super::intent::{extract_intent, Intent};
use super::types::{AudioFrame, VoiceConfig, VoiceError, VoiceState};
use super::vad::VoiceActivityDetector;
use super::wake_word::WakeWordDetector;
use super::whisper::WhisperEngine;
use crate::kernel::tts::TtsEngine;

/// Events emitted by the voice pipeline during a conversation loop.
#[derive(Debug)]
pub enum PipelineEvent {
    StateChanged(VoiceState),
    WakeWordDetected,
    Transcription(String),
    IntentRecognized(Intent),
    TtsComplete,
    Error(VoiceError),
}

/// Full voice pipeline: mic → VAD → Wake Word → ASR → Intent → TTS.
/// State machine: Idle → Listening → WakeDetected → Processing → Speaking → Listening.
pub struct VoicePipeline {
    state: VoiceState,
    config: VoiceConfig,
    vad: VoiceActivityDetector,
    wake_detector: WakeWordDetector,
    whisper: WhisperEngine,
    tts: TtsEngine,
    wake_active: bool,
}

impl VoicePipeline {
    pub fn new(config: VoiceConfig) -> Self {
        let vad = VoiceActivityDetector::new(config.vad_threshold);
        let wake = WakeWordDetector::new(
            &config.wake_word,
            config.vad_threshold,
            &config.whisper_model,
        );
        let whisper = WhisperEngine::new(&config.whisper_model, config.prefer_local);
        let tts = TtsEngine::new();
        Self {
            state: VoiceState::Idle,
            config,
            vad,
            wake_detector: wake,
            whisper,
            tts,
            wake_active: false,
        }
    }

    pub fn state(&self) -> VoiceState {
        self.state
    }

    pub fn config(&self) -> &VoiceConfig {
        &self.config
    }

    pub fn is_wake_active(&self) -> bool {
        self.wake_active
    }

    /// Start listening — transitions from Idle to Listening.
    pub fn start(&mut self) -> Result<(), VoiceError> {
        self.state = VoiceState::Listening;
        self.vad.reset();
        self.wake_detector.reset();
        self.wake_active = false;
        Ok(())
    }

    /// Stop the pipeline — reset all state.
    pub fn stop(&mut self) {
        self.state = VoiceState::Idle;
        self.vad.reset();
        self.wake_active = false;
    }

    /// Manually transition to WakeDetected (used by audio-based wake detection).
    pub fn set_wake_detected(&mut self) {
        if self.state == VoiceState::Listening {
            self.state = VoiceState::WakeDetected;
            self.wake_active = true;
        }
    }

    /// Process a single audio frame through the pipeline.
    /// Returns events produced during this frame's processing.
    pub fn process_frame(
        &mut self,
        frame: &AudioFrame,
    ) -> Result<Vec<PipelineEvent>, VoiceError> {
        if self.state == VoiceState::Idle {
            return Ok(vec![]);
        }

        let mut events = Vec::new();

        // Step 1: VAD — detect speech segments.
        let segment = self.vad.process(frame)?;
        let Some(segment) = segment else {
            return Ok(events);
        };

        // Step 2: Wake word check (if not yet activated).
        if !self.wake_active {
            let transcription = self.whisper.transcribe(&segment)?;
            if self.wake_detector.check_text(&transcription.text)? {
                self.wake_active = true;
                self.state = VoiceState::WakeDetected;
                events.push(PipelineEvent::StateChanged(VoiceState::WakeDetected));
                events.push(PipelineEvent::WakeWordDetected);
            }
            return Ok(events);
        }

        // Step 3: ASR — transcribe the full utterance.
        self.state = VoiceState::Processing;
        events.push(PipelineEvent::StateChanged(VoiceState::Processing));
        let transcription = self.whisper.transcribe(&segment)?;
        events.push(PipelineEvent::Transcription(transcription.text.clone()));

        // Step 4: Intent extraction.
        let intent = extract_intent(&transcription.text)?;
        events.push(PipelineEvent::IntentRecognized(intent));

        // Step 5: Reset for next utterance cycle.
        self.wake_active = false;
        self.state = VoiceState::Listening;
        events.push(PipelineEvent::StateChanged(VoiceState::Listening));

        Ok(events)
    }

    /// Speak a response via TTS (delegates to kernel TtsEngine).
    /// Returns synthesised WAV bytes on success. Restores previous state after.
    pub fn speak(&mut self, text: &str, locale: &str) -> Result<Vec<u8>, VoiceError> {
        let prev = self.state;
        self.state = VoiceState::Speaking;
        let result = self.tts.speak(text, locale).map_err(|e| {
            VoiceError::TtsError(e.to_string())
        });
        self.state = prev;
        result
    }

    /// Run the conversation loop from an audio frame receiver.
    /// Processes frames until the channel closes. Collects all events.
    pub async fn run_from_receiver(
        &mut self,
        mut rx: mpsc::UnboundedReceiver<AudioFrame>,
    ) -> Result<Vec<PipelineEvent>, VoiceError> {
        self.start()?;
        let mut all_events = vec![PipelineEvent::StateChanged(VoiceState::Listening)];

        while let Some(frame) = rx.recv().await {
            match self.process_frame(&frame) {
                Ok(events) => all_events.extend(events),
                Err(e) => {
                    all_events.push(PipelineEvent::Error(
                        VoiceError::PipelineError(e.to_string()),
                    ));
                }
            }
        }

        self.stop();
        all_events.push(PipelineEvent::StateChanged(VoiceState::Idle));
        Ok(all_events)
    }

    /// Run the full conversation loop with live AudioCapture.
    /// Returns a channel of events and a handle to stop the pipeline.
    #[cfg(feature = "voice")]
    pub fn run_live(
        &mut self,
    ) -> Result<
        (
            mpsc::UnboundedReceiver<PipelineEvent>,
            super::audio_capture::AudioCapture,
        ),
        VoiceError,
    > {
        use super::audio_capture::{AudioCapture, CaptureConfig};

        let capture_cfg = CaptureConfig::default();
        let mut capture = AudioCapture::new(capture_cfg);
        let audio_rx = capture.start()?;

        self.start()?;
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        if let Err(e) = event_tx.send(PipelineEvent::StateChanged(VoiceState::Listening)) {
            tracing::warn!("voice pipeline: initial state send: {e}");
        }

        // Dedicated thread — VoicePipeline contains non-Send types (whisper-rs).
        let config = self.config.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("voice pipeline runtime");
            rt.block_on(async move {
                let mut inner = VoicePipeline::new(config);
                if let Err(e) = inner.start() {
                    tracing::warn!("voice pipeline: inner start: {e}");
                }
                let mut audio_rx = audio_rx;
                while let Some(frame) = audio_rx.recv().await {
                    match inner.process_frame(&frame) {
                        Ok(events) => {
                            for ev in events {
                                if event_tx.send(ev).is_err() {
                                    return;
                                }
                            }
                        }
                        Err(e) => {
                            if let Err(se) = event_tx.send(PipelineEvent::Error(
                                VoiceError::PipelineError(e.to_string()),
                            )) {
                                tracing::warn!("voice pipeline: error event send: {se}");
                            }
                        }
                    }
                }
                if let Err(e) = event_tx.send(PipelineEvent::StateChanged(VoiceState::Idle)) {
                    tracing::warn!("voice pipeline: idle state send: {e}");
                }
            });
        });

        Ok((event_rx, capture))
    }
}
