use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Audio analyzer that captures input and computes RMS/peak values
pub struct AudioAnalyzer {
    _stream: cpal::Stream,
    /// RMS value (0.0 - 1.0) stored as bits for atomic access
    rms_bits: Arc<AtomicU32>,
    /// Peak value (0.0 - 1.0) stored as bits
    peak_bits: Arc<AtomicU32>,
    /// Low frequency energy (bass)
    bass_bits: Arc<AtomicU32>,
    /// Bass energy from previous frame for kick detection
    prev_bass: f32,
    /// Kick detection threshold
    kick_threshold: f32,
}

impl AudioAnalyzer {
    pub fn new(device_index: Option<usize>) -> Result<Self, String> {
        let host = cpal::default_host();

        // List available input devices
        let devices: Vec<_> = host
            .input_devices()
            .map_err(|e| format!("Failed to enumerate audio devices: {}", e))?
            .collect();

        if devices.is_empty() {
            return Err("No audio input devices found".to_string());
        }

        // List devices
        for (i, device) in devices.iter().enumerate() {
            let name = device.name().unwrap_or_else(|_| "Unknown".to_string());
            log::info!("Audio input {}: {}", i, name);
        }

        // Select device
        let device = if let Some(idx) = device_index {
            devices.into_iter().nth(idx).ok_or_else(|| {
                format!("Audio device {} not found", idx)
            })?
        } else {
            host.default_input_device()
                .ok_or_else(|| "No default audio input device".to_string())?
        };

        let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        log::info!("Using audio input: {}", device_name);

        let config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get audio config: {}", e))?;

        log::info!(
            "Audio config: {} channels, {} Hz",
            config.channels(),
            config.sample_rate().0
        );

        let rms_bits = Arc::new(AtomicU32::new(0));
        let peak_bits = Arc::new(AtomicU32::new(0));
        let bass_bits = Arc::new(AtomicU32::new(0));

        let rms_bits_clone = rms_bits.clone();
        let peak_bits_clone = peak_bits.clone();
        let bass_bits_clone = bass_bits.clone();

        let channels = config.channels() as usize;
        let sample_rate = config.sample_rate().0 as f32;

        // Simple low-pass filter state for bass extraction
        let mut bass_filter_state = 0.0f32;
        let bass_cutoff = 150.0; // Hz
        let bass_alpha = (2.0 * std::f32::consts::PI * bass_cutoff / sample_rate)
            / (2.0 * std::f32::consts::PI * bass_cutoff / sample_rate + 1.0);

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    let mut sum_sq = 0.0f32;
                    let mut peak = 0.0f32;
                    let mut bass_sum = 0.0f32;

                    // Process samples (mix down to mono)
                    for chunk in data.chunks(channels) {
                        let sample: f32 = chunk.iter().sum::<f32>() / channels as f32;
                        sum_sq += sample * sample;
                        peak = peak.max(sample.abs());

                        // Simple low-pass filter for bass
                        bass_filter_state = bass_alpha * sample + (1.0 - bass_alpha) * bass_filter_state;
                        bass_sum += bass_filter_state * bass_filter_state;
                    }

                    let num_samples = data.len() / channels;
                    if num_samples > 0 {
                        let rms = (sum_sq / num_samples as f32).sqrt();
                        let bass_rms = (bass_sum / num_samples as f32).sqrt() * 4.0; // Boost bass

                        // Smooth values (exponential moving average)
                        let old_rms = f32::from_bits(rms_bits_clone.load(Ordering::Relaxed));
                        let old_peak = f32::from_bits(peak_bits_clone.load(Ordering::Relaxed));
                        let old_bass = f32::from_bits(bass_bits_clone.load(Ordering::Relaxed));

                        let smoothed_rms = old_rms * 0.8 + rms * 0.2;
                        let smoothed_peak = old_peak * 0.7 + peak * 0.3; // Faster attack for peak
                        let smoothed_bass = old_bass * 0.85 + bass_rms * 0.15;

                        rms_bits_clone.store(smoothed_rms.to_bits(), Ordering::Relaxed);
                        peak_bits_clone.store(smoothed_peak.to_bits(), Ordering::Relaxed);
                        bass_bits_clone.store(smoothed_bass.to_bits(), Ordering::Relaxed);
                    }
                },
                |err| {
                    log::error!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to build audio stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start audio stream: {}", e))?;

        log::info!("Audio capture started");

        Ok(Self {
            _stream: stream,
            rms_bits,
            peak_bits,
            bass_bits,
            prev_bass: 0.0,
            kick_threshold: 0.15, // Sensitivity for kick detection
        })
    }

    /// Get current RMS value (0.0 - 1.0, typically 0.0 - 0.5 for normal audio)
    pub fn rms(&self) -> f32 {
        f32::from_bits(self.rms_bits.load(Ordering::Relaxed)).min(1.0)
    }

    /// Get current peak value (0.0 - 1.0)
    pub fn peak(&self) -> f32 {
        f32::from_bits(self.peak_bits.load(Ordering::Relaxed)).min(1.0)
    }

    /// Get bass energy (0.0 - 1.0, boosted low frequencies)
    pub fn bass(&self) -> f32 {
        f32::from_bits(self.bass_bits.load(Ordering::Relaxed)).min(1.0)
    }

    /// Detect if a kick/transient occurred (call once per frame)
    /// Returns the kick intensity (0.0 if no kick, > 0.0 if kick detected)
    pub fn detect_kick(&mut self) -> f32 {
        let current_bass = self.bass();
        let delta = current_bass - self.prev_bass;
        self.prev_bass = current_bass;

        // Kick detected if bass energy increased significantly
        if delta > self.kick_threshold {
            delta * 2.0 // Return intensity scaled
        } else {
            0.0
        }
    }
}

/// List available audio input devices
pub fn list_audio_devices() -> Vec<String> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devices| {
            devices
                .map(|d| d.name().unwrap_or_else(|_| "Unknown".to_string()))
                .collect()
        })
        .unwrap_or_default()
}
