# ADR-0123: Jarvis Voice Engine

**Status**: Accepted
**Date**: 29 Marzo 2026

## Context

Jarvis (the local kernel) handles text-based Telegram interaction and TTS via macOS `say`. To enable hands-free voice control of Convergio, we need a full voice pipeline: microphone capture, voice activity detection (VAD), speech-to-text (STT), wake word detection, intent classification, and neural TTS. All inference must run locally on Apple Silicon (M1 Pro / M5 Max) with no cloud dependency.

## Decision

Implement a feature-gated `voice` module in the daemon with five stages:

| Stage | Crate / Model | Purpose |
|---|---|---|
| Audio capture | `cpal` | Native microphone input, stereo-to-mono, linear resampling to 16kHz |
| VAD | `webrtc-vad` | libwebrtc-based speech onset/offset detection (~10ms frames) |
| STT | `whisper-rs` (GGML) | Local Whisper inference for transcription |
| Wake word | VAD + Whisper micro-transcription | Detect "convergio" in short speech bursts |
| TTS | Voxtral 4B (`mlx-community/Voxtral-4B-TTS-2603-mlx-4bit`) via `mlx-audio` | Neural Italian/English speech synthesis |
| Intent | Rule-based classifier | Map transcribed text to `cvg` commands, queries, navigation |

### Key choices

1. **Feature gate** (`voice` in Cargo.toml) — opt-in, not default. Heavy native dependencies (cpal, webrtc-vad, whisper-rs) are only compiled when needed. Modules that don't require native audio (types, intent, wake_word text path) compile without the flag.

2. **webrtc-vad over silero-vad** — webrtc-vad is a pure C library with Rust bindings, no Python/ONNX runtime needed. Proven in production (Chrome/WebRTC). Four aggressiveness levels mapped from a 0.0–1.0 threshold.

3. **whisper-rs over candle-whisper** — whisper-rs wraps whisper.cpp (GGML), which is battle-tested on Apple Silicon with Metal acceleration. Lazy-loads the model on first transcription to avoid startup cost.

4. **Voxtral 4B over Kokoro/macOS say** — Voxtral produces natural multilingual speech (Italian primary, English secondary). MLX 4-bit quantization runs on Apple Silicon Neural Engine. Installed from `mlx-audio` git main because PyPI 0.4.1 lacks `voxtral_tts` support.

5. **cpal for audio capture** — cross-platform, no Python dependency. Handles stereo-to-mono conversion and linear resampling in the capture callback. Frame buffering ensures VAD receives exact 10/20/30ms chunks.

6. **Pipeline state machine** — `Idle → Listening → WakeDetected → Processing → Speaking` ensures clean transitions and prevents concurrent inference.

## Consequences

- `cargo build` without `--features voice` is unaffected (zero added compile time).
- `cargo build --features voice` adds cpal, webrtc-vad, whisper-rs, hound, ringbuf dependencies.
- Whisper GGML model (~500 MB for "small") must be present at `~/.cache/whisper/ggml-small.bin`.
- Voxtral 4B model (~2.5 GB) must be cached via `mlx-audio` / HuggingFace.
- `mlx-audio` must be installed from git main until PyPI catches up.
- Future: streaming VAD-to-ASR pipeline, Whisper API fallback, voice-activated plan execution.

## Modules

| File | Lines | Purpose |
|---|---|---|
| `voice/types.rs` | 92 | VoiceState, VoiceConfig, VoiceError, AudioFrame, SpeechSegment |
| `voice/vad.rs` | 119 | VoiceActivityDetector (webrtc-vad), threshold mapping |
| `voice/whisper.rs` | 214 | WhisperEngine with lazy GGML loading, native inference |
| `voice/wake_word.rs` | 83 | WakeWordDetector (VAD + micro-transcription) |
| `voice/audio_capture.rs` | 244 | cpal capture, stereo-to-mono, resampling, frame buffering |
| `voice/audio_util.rs` | 32 | stereo_to_mono, linear resample helpers |
| `voice/intent.rs` | 82 | Intent extraction, rule-based classifier |
| `voice/pipeline.rs` | 113 | VoicePipeline state machine (VAD → Wake → ASR → Intent → TTS) |
| `voice/mod.rs` | 42 | Module wiring, feature-gated re-exports |
