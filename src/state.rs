use crate::mesh::MeshType;
use crate::midi::MidiCommand;
use crate::p_lock::PLockSystem;

/// Maximum number of concurrent ripples
pub const MAX_RIPPLES: usize = 4;

/// A single ripple effect (concentric wave)
#[derive(Clone, Copy, Default)]
pub struct Ripple {
    /// Center position X (0.0 - 1.0, normalized)
    pub x: f32,
    /// Center position Y (0.0 - 1.0, normalized)
    pub y: f32,
    /// Current radius (expands over time)
    pub radius: f32,
    /// Intensity (fades over time)
    pub intensity: f32,
    /// Is this ripple active?
    pub active: bool,
}

impl Ripple {
    /// Convert to array for shader uniform [x, y, radius, intensity]
    pub fn to_array(&self) -> [f32; 4] {
        [self.x, self.y, self.radius, self.intensity]
    }
}

/// Manages multiple ripple effects
pub struct RippleSystem {
    pub ripples: [Ripple; MAX_RIPPLES],
    next_index: usize,
    /// Expansion speed
    pub expansion_rate: f32,
    /// Fade rate
    pub fade_rate: f32,
}

impl Default for RippleSystem {
    fn default() -> Self {
        Self {
            ripples: [Ripple::default(); MAX_RIPPLES],
            next_index: 0,
            expansion_rate: 0.02,  // How fast ripples expand
            fade_rate: 0.02,      // How fast ripples fade
        }
    }
}

impl RippleSystem {
    /// Spawn a new ripple at a random or specified position
    pub fn spawn(&mut self, x: f32, y: f32, intensity: f32) {
        self.ripples[self.next_index] = Ripple {
            x,
            y,
            radius: 0.0,
            intensity,
            active: true,
        };
        self.next_index = (self.next_index + 1) % MAX_RIPPLES;
    }

    /// Spawn ripple at random position
    pub fn spawn_random(&mut self, intensity: f32) {
        // Simple pseudo-random using time-based seed
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let x = ((t % 1000) as f32) / 1000.0;
        let y = (((t / 1000) % 1000) as f32) / 1000.0;
        self.spawn(x, y, intensity);
    }

    /// Update all ripples (call each frame)
    pub fn update(&mut self) {
        for ripple in &mut self.ripples {
            if ripple.active {
                ripple.radius += self.expansion_rate;
                ripple.intensity -= self.fade_rate;
                if ripple.intensity <= 0.0 {
                    ripple.active = false;
                    ripple.intensity = 0.0;
                }
            }
        }
    }
}

/// All application state / parameters
pub struct AppState {
    // Display
    pub width: u32,
    pub height: u32,

    // LFO arguments (phase accumulators)
    pub x_lfo_arg: f32,
    pub y_lfo_arg: f32,
    pub z_lfo_arg: f32,

    // LFO shapes (0=sine, 1=square, 2=saw, 3=noise)
    pub x_lfo_shape: i32,
    pub y_lfo_shape: i32,
    pub z_lfo_shape: i32,

    // Ring modulation switches
    pub x_ringmod: bool,
    pub y_ringmod: bool,
    pub z_ringmod: bool,

    // Phase modulation switches
    pub x_phasemod: bool,
    pub y_phasemod: bool,
    pub z_phasemod: bool,

    // Frequency zero switches
    pub x_freq0: bool,
    pub y_freq0: bool,
    pub z_freq0: bool,

    // Visual switches
    pub wireframe: bool,
    pub bright_switch: bool,
    pub invert: bool,
    pub greyscale: bool,
    pub luma_switch: bool,

    // Mesh
    pub mesh_type: MeshType,
    pub scale: u32,

    // Transforms
    pub global_x_displace: f32,
    pub global_y_displace: f32,
    pub rotate_x: f32,
    pub rotate_y: f32,
    pub rotate_z: f32,

    // Stroke
    pub stroke_weight: f32,

    // Keyboard offsets
    pub keyboard_offsets: KeyboardOffsets,

    // Parameter lock system
    pub p_lock: PLockSystem,

    // Audio modulation values
    pub audio_mod_displacement: f32,
    pub audio_mod_lfo: f32,
    pub audio_mod_z: f32,

    // Audio wave effect - undulating lines
    pub audio_wave_phase: f32,
    pub audio_wave_amp: f32,
    pub audio_wave_freq: f32,
}

#[derive(Default)]
pub struct KeyboardOffsets {
    pub az: f32,
    pub sx: f32,
    pub dc: f32,
    pub fv: f32,
    pub gb: f32,
    pub hn: f32,
    pub jm: f32,
    pub kk: f32,
    pub ll: f32,
    pub ylfo_amp: f32,
    pub ty: f32,
    pub ui: f32,
    pub op: f32,
    pub er: f32,
    pub qw: f32,
    pub scale_key: i32,
}

impl AppState {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            x_lfo_arg: 0.0,
            y_lfo_arg: 0.0,
            z_lfo_arg: 0.0,
            x_lfo_shape: 0,
            y_lfo_shape: 0,
            z_lfo_shape: 0,
            x_ringmod: false,
            y_ringmod: false,
            z_ringmod: false,
            x_phasemod: false,
            y_phasemod: false,
            z_phasemod: false,
            x_freq0: false,
            y_freq0: false,
            z_freq0: false,
            wireframe: false,
            bright_switch: false,
            invert: false,
            greyscale: false,
            luma_switch: false,
            mesh_type: MeshType::Triangles,
            scale: 64,
            global_x_displace: 0.0,
            global_y_displace: 0.0,
            rotate_x: 0.0,
            rotate_y: 0.0,
            rotate_z: 0.0,
            stroke_weight: 1.0,
            keyboard_offsets: KeyboardOffsets::default(),
            p_lock: PLockSystem::new(),
            audio_mod_displacement: 0.0,
            audio_mod_lfo: 0.0,
            audio_mod_z: 0.0,
            audio_wave_phase: 0.0,
            audio_wave_amp: 0.0,
            audio_wave_freq: 15.0, // Base wave frequency
        }
    }

    /// Process a MIDI command and update state accordingly
    pub fn process_midi(&mut self, cmd: MidiCommand) {
        const THRESHOLD: f32 = 0.04;

        match cmd {
            MidiCommand::LumaKeyLevel(v) => self.p_lock.set_with_latch(0, v, THRESHOLD),
            MidiCommand::DisplaceX(v) => self.p_lock.set_with_latch(1, v, THRESHOLD),
            MidiCommand::DisplaceY(v) => self.p_lock.set_with_latch(2, v, THRESHOLD),
            MidiCommand::ZFrequency(v) => self.p_lock.set_with_latch(3, v, THRESHOLD),
            MidiCommand::XFrequency(v) => self.p_lock.set_with_latch(4, v, THRESHOLD),
            MidiCommand::YFrequency(v) => self.p_lock.set_with_latch(5, v, THRESHOLD),
            MidiCommand::Zoom(v) => self.p_lock.set_with_latch(6, v, THRESHOLD),
            MidiCommand::Scale(v) => self.p_lock.set_with_latch(7, v, THRESHOLD),
            MidiCommand::CenterX(v) => self.p_lock.set_with_latch(8, v, THRESHOLD),
            MidiCommand::CenterY(v) => self.p_lock.set_with_latch(9, v, THRESHOLD),
            MidiCommand::ZLfoArg(v) => self.p_lock.set_with_latch(10, v, THRESHOLD),
            MidiCommand::ZLfoAmp(v) => self.p_lock.set_with_latch(11, v, THRESHOLD),
            MidiCommand::XLfoArg(v) => self.p_lock.set_with_latch(12, v, THRESHOLD),
            MidiCommand::XLfoAmp(v) => self.p_lock.set_with_latch(13, v, THRESHOLD),
            MidiCommand::YLfoArg(v) => self.p_lock.set_with_latch(14, v, THRESHOLD),
            MidiCommand::YLfoAmp(v) => self.p_lock.set_with_latch(15, v, THRESHOLD),

            MidiCommand::RecordStart => self.p_lock.start_recording(),
            MidiCommand::RecordStop => self.p_lock.stop_recording(),
            MidiCommand::Reset => {
                self.p_lock.clear();
                self.global_x_displace = 0.0;
                self.global_y_displace = 0.0;
                self.rotate_x = 0.0;
                self.rotate_y = 0.0;
                self.rotate_z = 0.0;
            }

            MidiCommand::ZLfoShape(s) => self.z_lfo_shape = s,
            MidiCommand::XLfoShape(s) => self.x_lfo_shape = s,
            MidiCommand::YLfoShape(s) => self.y_lfo_shape = s,

            MidiCommand::ZRingMod(v) => self.z_ringmod = v,
            MidiCommand::XRingMod(v) => self.x_ringmod = v,
            MidiCommand::YRingMod(v) => self.y_ringmod = v,
            MidiCommand::ZPhaseMod(v) => self.z_phasemod = v,
            MidiCommand::XPhaseMod(v) => self.x_phasemod = v,
            MidiCommand::YPhaseMod(v) => self.y_phasemod = v,

            MidiCommand::ZFreqZero(v) => self.z_freq0 = v,
            MidiCommand::XFreqZero(v) => self.x_freq0 = v,
            MidiCommand::YFreqZero(v) => self.y_freq0 = v,

            MidiCommand::SetTriangleMesh => {
                self.mesh_type = MeshType::Triangles;
                self.wireframe = false;
            }
            MidiCommand::SetHorizontalLines => self.mesh_type = MeshType::HorizontalLines,
            MidiCommand::SetVerticalLines => self.mesh_type = MeshType::VerticalLines,
            MidiCommand::SetWireframe => {
                self.mesh_type = MeshType::Triangles;
                self.wireframe = true;
            }

            MidiCommand::Greyscale(v) => self.greyscale = v,
            MidiCommand::Invert(v) => self.invert = v,
            MidiCommand::BrightSwitch(v) => self.bright_switch = v,
            MidiCommand::StrokeWeight(v) => self.stroke_weight = v,

            MidiCommand::RotateX(v) => self.rotate_x = v,
            MidiCommand::RotateY(v) => self.rotate_y = v,
            MidiCommand::RotateZ(v) => self.rotate_z = v,
            MidiCommand::GlobalXDisplace(v) => {
                if !v {
                    self.global_x_displace = 0.0;
                }
            }
            MidiCommand::GlobalYDisplace(v) => {
                if !v {
                    self.global_y_displace = 0.0;
                }
            }
            _ => {}
        }
    }

    /// Calculate derived parameters for rendering
    /// All values are in clip space (-1 to 1) for the WGSL shader
    pub fn calculate_render_params(&self) -> RenderParams {
        let ko = &self.keyboard_offsets;

        RenderParams {
            // Luma key threshold (0 to 1)
            luma_key_level: self.p_lock.get(0) + 0.1 * ko.az,
            // Displacement: small values in clip space (0.0 to ~0.5 max)
            displace_x: 0.5 * (self.p_lock.get(1) + ko.qw),
            displace_y: 0.5 * (self.p_lock.get(2) + ko.er),
            // Spatial frequencies for LFO (how many waves across the mesh)
            z_frequency: 10.0 * self.p_lock.get(3) + ko.sx,
            x_frequency: 10.0 * self.p_lock.get(4) + ko.gb,
            y_frequency: 10.0 * self.p_lock.get(5) + ko.kk,
            // Zoom (not used in clip space shader, but keep for mesh scale)
            zoom: self.p_lock.get(6) + ko.op,
            // Grid density (1 to 127)
            scale: ((1.0 - self.p_lock.get(7)) * 126.0 + 1.0 + ko.scale_key as f32) as u32,
            // Center offset in clip space (-1 to 1)
            center_x: 2.0 * (self.p_lock.get(8) - 0.5) + 0.1 * ko.ty,
            center_y: 2.0 * (self.p_lock.get(9) - 0.5) + 0.1 * ko.ui,
            // LFO phase increment (controls animation speed)
            z_lfo_arg: self.p_lock.get(10) + ko.dc,
            // LFO amplitude in clip space (small values!)
            z_lfo_amp: 0.1 * self.p_lock.get(11) + 0.01 * ko.fv,
            x_lfo_arg: self.p_lock.get(12) + ko.hn,
            x_lfo_amp: 0.2 * self.p_lock.get(13) + 0.01 * ko.jm + 0.1 * self.audio_mod_lfo,
            y_lfo_arg: self.p_lock.get(14) + ko.ll,
            y_lfo_amp: 0.2 * self.p_lock.get(15) + 0.01 * ko.ylfo_amp + 0.1 * self.audio_mod_lfo,
            // Audio modulation (small values for clip space)
            audio_displacement: 0.1 * self.audio_mod_displacement,
            audio_z: 0.05 * self.audio_mod_z,
        }
    }
}

pub struct RenderParams {
    pub luma_key_level: f32,
    pub displace_x: f32,
    pub displace_y: f32,
    pub z_frequency: f32,
    pub x_frequency: f32,
    pub y_frequency: f32,
    pub zoom: f32,
    pub scale: u32,
    pub center_x: f32,
    pub center_y: f32,
    pub z_lfo_arg: f32,
    pub z_lfo_amp: f32,
    pub x_lfo_arg: f32,
    pub x_lfo_amp: f32,
    pub y_lfo_arg: f32,
    pub y_lfo_amp: f32,
    pub audio_displacement: f32,
    pub audio_z: f32,
}
