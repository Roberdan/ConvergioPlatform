use super::audio_capture::{AudioCapture, CaptureConfig, CaptureEvent};

#[test]
fn default_config_uses_16khz_mono() {
    let cfg = CaptureConfig::default();
    assert_eq!(cfg.sample_rate, 16000);
    assert_eq!(cfg.channels, 1);
    assert_eq!(cfg.frame_duration_ms, 10);
}

#[test]
fn frame_size_calculation() {
    let cfg = CaptureConfig::default();
    // 16kHz * 10ms = 160 samples per frame
    assert_eq!(cfg.frame_size(), 160);

    let cfg30 = CaptureConfig {
        frame_duration_ms: 30,
        ..Default::default()
    };
    // 16kHz * 30ms = 480 samples per frame
    assert_eq!(cfg30.frame_size(), 480);
}

#[test]
fn capture_creates_with_config() {
    let cfg = CaptureConfig::default();
    let capture = AudioCapture::new(cfg.clone());
    assert!(!capture.is_running());
    assert_eq!(capture.config().sample_rate, cfg.sample_rate);
}

#[test]
fn capture_event_display() {
    let ev = CaptureEvent::Error("device lost".to_string());
    assert!(format!("{ev}").contains("device lost"));

    let ev2 = CaptureEvent::DeviceChanged;
    assert_eq!(format!("{ev2}"), "device changed");
}

#[test]
fn capture_start_stop_without_device() {
    // On CI without audio hardware, start() should return an error.
    let mut capture = AudioCapture::new(CaptureConfig::default());
    let rx = capture.start();
    // Either we get a receiver (hardware present) or an error.
    match rx {
        Ok(_rx) => {
            // If hardware is present, stop should work.
            capture.stop();
            assert!(!capture.is_running());
        }
        Err(e) => {
            // Expected on CI — no audio device.
            let msg = e.to_string();
            assert!(
                msg.contains("audio") || msg.contains("device") || msg.contains("host"),
                "unexpected error: {msg}"
            );
        }
    }
}

#[test]
fn stereo_to_mono_conversion() {
    use super::audio_capture::stereo_to_mono;
    let stereo = vec![100i16, 200, 300, 400, 500, 600];
    let mono = stereo_to_mono(&stereo);
    assert_eq!(mono.len(), 3);
    assert_eq!(mono[0], 150); // (100+200)/2
    assert_eq!(mono[1], 350); // (300+400)/2
    assert_eq!(mono[2], 550); // (500+600)/2
}

#[test]
fn stereo_to_mono_empty() {
    use super::audio_capture::stereo_to_mono;
    let mono = stereo_to_mono(&[]);
    assert!(mono.is_empty());
}

#[test]
fn resample_passthrough_at_target_rate() {
    use super::audio_capture::resample;
    let samples = vec![1i16, 2, 3, 4, 5];
    let out = resample(&samples, 16000, 16000);
    assert_eq!(out, samples);
}

#[test]
fn resample_48k_to_16k() {
    use super::audio_capture::resample;
    // 48kHz → 16kHz is 3:1 ratio. 9 samples → 3 output samples.
    let samples: Vec<i16> = (0..9).map(|i| i * 100).collect();
    let out = resample(&samples, 48000, 16000);
    assert_eq!(out.len(), 3);
}
