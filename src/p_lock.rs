/// Parameter Lock system for recording and playing back parameter automation
/// Ported from the original spectral_mesh p_lock implementation

pub const P_LOCK_SIZE: usize = 240;
pub const P_LOCK_NUMBER: usize = 17;

pub struct PLockSystem {
    /// 2D array of parameter values [param_index][step]
    locks: [[f32; P_LOCK_SIZE]; P_LOCK_NUMBER],
    /// Smoothed output values for each parameter
    smoothed: [f32; P_LOCK_NUMBER],
    /// MIDI active flags for latching behavior
    midi_active: [bool; P_LOCK_NUMBER],
    /// Current step position
    increment: usize,
    /// Recording enabled flag
    pub recording: bool,
    /// Smoothing factor (0.0 - 1.0)
    pub smooth_factor: f32,
}

impl PLockSystem {
    pub fn new() -> Self {
        let mut system = Self {
            locks: [[0.0; P_LOCK_SIZE]; P_LOCK_NUMBER],
            smoothed: [0.0; P_LOCK_NUMBER],
            midi_active: [false; P_LOCK_NUMBER],
            increment: 0,
            recording: false,
            smooth_factor: 0.5,
        };

        // Set initial default values for effects to be visible
        // Index mapping from state.rs:
        // 0: luma_key_level, 1: displace_x, 2: displace_y
        // 3: z_frequency, 4: x_frequency, 5: y_frequency
        // 6: zoom, 7: scale
        // 8: center_x, 9: center_y (0.5 = centered)
        // 10: z_lfo_arg, 11: z_lfo_amp
        // 12: x_lfo_arg, 13: x_lfo_amp
        // 14: y_lfo_arg, 15: y_lfo_amp

        // Default luma key level (0.5 = mid-brightness threshold)
        system.set_all(0, 0.5);

        // Displacement - brightness-based distortion
        // These get multiplied by 0.5 in calculate_render_params
        system.set_all(1, 0.1);  // displace_x -> 0.05 in clip space
        system.set_all(2, 0.1);  // displace_y -> 0.05 in clip space

        // LFO spatial frequencies (get multiplied by 10.0)
        system.set_all(3, 0.2);  // z_frequency -> 2.0 waves
        system.set_all(4, 0.3);  // x_frequency -> 3.0 waves
        system.set_all(5, 0.3);  // y_frequency -> 3.0 waves

        // Center position (0.5 = centered, gets converted to 0.0 in clip space)
        system.set_all(8, 0.5);  // center_x
        system.set_all(9, 0.5);  // center_y

        // LFO phase increments (animation speed, accumulated each frame)
        system.set_all(10, 0.02); // z_lfo_arg
        system.set_all(12, 0.015); // x_lfo_arg
        system.set_all(14, 0.018); // y_lfo_arg

        // LFO amplitudes (get multiplied by 0.1-0.2 in calculate_render_params)
        system.set_all(11, 0.2); // z_lfo_amp -> 0.02 in clip space
        system.set_all(13, 0.3); // x_lfo_amp -> 0.06 in clip space
        system.set_all(15, 0.3); // y_lfo_amp -> 0.06 in clip space

        // Scale (0.5 = mid-scale grid density of ~64)
        system.set_all(7, 0.5);

        system
    }

    /// Set value for all steps of a parameter
    pub fn set_all(&mut self, index: usize, value: f32) {
        if index < P_LOCK_NUMBER {
            for j in 0..P_LOCK_SIZE {
                self.locks[index][j] = value;
            }
            self.smoothed[index] = value;
        }
    }

    /// Clear all parameter locks
    pub fn clear(&mut self) {
        for i in 0..P_LOCK_NUMBER {
            for j in 0..P_LOCK_SIZE {
                self.locks[i][j] = 0.0;
            }
            self.smoothed[i] = 0.0;
            self.midi_active[i] = false;
        }
        self.increment = 0;
    }

    /// Update smoothed values and advance step if recording
    pub fn update(&mut self) {
        for i in 0..P_LOCK_NUMBER {
            // Apply smoothing: new = current * (1 - smooth) + old * smooth
            self.smoothed[i] = self.locks[i][self.increment] * (1.0 - self.smooth_factor)
                + self.smoothed[i] * self.smooth_factor;

            // Zero out very small values to prevent floating point accumulation
            if self.smoothed[i].abs() < 0.01 {
                self.smoothed[i] = 0.0;
            }
        }

        if self.recording {
            self.increment = (self.increment + 1) % P_LOCK_SIZE;
        }
    }

    /// Get smoothed value for a parameter
    pub fn get(&self, index: usize) -> f32 {
        if index < P_LOCK_NUMBER {
            self.smoothed[index]
        } else {
            0.0
        }
    }

    /// Set value at current step for a parameter (with MIDI latching)
    pub fn set_with_latch(&mut self, index: usize, value: f32, threshold: f32) {
        if index >= P_LOCK_NUMBER {
            return;
        }

        let current = self.locks[index][self.increment];
        let diff = (value - current).abs();

        // Latch behavior: only activate if value is close to current
        if diff < threshold {
            self.midi_active[index] = true;
        }

        if self.midi_active[index] {
            self.locks[index][self.increment] = value;
        }
    }

    /// Set value directly without latching
    pub fn set(&mut self, index: usize, value: f32) {
        if index < P_LOCK_NUMBER {
            self.locks[index][self.increment] = value;
        }
    }

    /// Reset MIDI active state for a parameter
    pub fn reset_midi_active(&mut self, index: usize) {
        if index < P_LOCK_NUMBER {
            self.midi_active[index] = false;
        }
    }

    /// Start recording - copies current step to all steps
    pub fn start_recording(&mut self) {
        self.recording = true;
        for i in 0..P_LOCK_NUMBER {
            self.smoothed[i] = 0.0;
            let current_value = self.locks[i][self.increment];
            for j in 0..P_LOCK_SIZE {
                self.locks[i][j] = current_value;
            }
        }
    }

    /// Stop recording
    pub fn stop_recording(&mut self) {
        self.recording = false;
    }

    /// Get current step
    pub fn current_step(&self) -> usize {
        self.increment
    }
}

impl Default for PLockSystem {
    fn default() -> Self {
        Self::new()
    }
}
