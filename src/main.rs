mod audio;
mod mesh;
mod midi;
mod noise;
mod p_lock;
mod renderer;
mod state;
mod video;

use audio::AudioAnalyzer;
use clap::Parser;
use mesh::Mesh;
use midi::MidiHandler;
use noise::NoiseBank;
use renderer::Renderer;
use state::AppState;
use video::{DummyVideoSource, VideoCapture};
use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

/// Spectral Mesh - Real-time video mesh distortion
#[derive(Parser, Debug)]
#[command(name = "spectral_mesh")]
#[command(version = "5.0")]
#[command(about = "Real-time audiovisual mesh distortion with MIDI control")]
struct Args {
    /// MIDI input device index
    #[arg(short, long, default_value_t = 1)]
    midi: usize,

    /// Video input device index
    #[arg(short, long, default_value_t = 0)]
    video: u32,

    /// Video processing width (lower = faster, use 16:9 for modern cameras)
    #[arg(long, default_value_t = 960)]
    width: u32,

    /// Video processing height (lower = faster, use 16:9 for modern cameras)
    #[arg(long, default_value_t = 540)]
    height: u32,

    /// Audio input device index (optional, omit to disable)
    #[arg(short, long)]
    audio: Option<usize>,

    /// List available devices and exit
    #[arg(long)]
    list_devices: bool,

    /// Window width
    #[arg(long, default_value_t = 1280)]
    window_width: u32,

    /// Window height
    #[arg(long, default_value_t = 720)]
    window_height: u32,
}

const NOISE_WIDTH: u32 = 180;
const NOISE_HEIGHT: u32 = 120;

enum VideoSource {
    Camera(VideoCapture),
    Dummy(DummyVideoSource),
}

struct App {
    renderer: Renderer,
    state: AppState,
    midi: Option<MidiHandler>,
    noise_bank: NoiseBank,
    video_source: VideoSource,
    audio: Option<AudioAnalyzer>,
    last_mesh_scale: u32,
    needs_mesh_rebuild: bool,
    show_help: bool,
    video_width: u32,
    video_height: u32,
}

impl App {
    fn new(renderer: Renderer, args: &Args) -> Self {
        // Initialize MIDI
        let midi = match MidiHandler::new(args.midi) {
            Ok(midi) => {
                log::info!("MIDI initialized on port {}", args.midi);
                Some(midi)
            }
            Err(e) => {
                log::warn!("MIDI initialization failed: {}", e);
                None
            }
        };

        // Try to initialize camera, fall back to dummy if it fails
        let video_source = match VideoCapture::new(args.width, args.height, args.video) {
            Ok(cam) => {
                log::info!("Camera {} initialized ({}x{})", args.video, args.width, args.height);
                VideoSource::Camera(cam)
            }
            Err(e) => {
                log::warn!("Camera failed: {}. Using test pattern.", e);
                VideoSource::Dummy(DummyVideoSource::new(args.width, args.height))
            }
        };

        // Initialize audio if requested
        let audio = if let Some(audio_idx) = args.audio {
            match AudioAnalyzer::new(Some(audio_idx)) {
                Ok(analyzer) => {
                    log::info!("Audio analyzer initialized");
                    Some(analyzer)
                }
                Err(e) => {
                    log::warn!("Audio initialization failed: {}", e);
                    None
                }
            }
        } else {
            // Try default audio device
            match AudioAnalyzer::new(None) {
                Ok(analyzer) => {
                    log::info!("Audio analyzer initialized (default device)");
                    Some(analyzer)
                }
                Err(e) => {
                    log::info!("No audio input: {}", e);
                    None
                }
            }
        };

        log::info!("Spectral Mesh initialized");
        log::info!("Press H for help");

        Self {
            renderer,
            state: AppState::new(args.width, args.height),
            midi,
            noise_bank: NoiseBank::new(NOISE_WIDTH, NOISE_HEIGHT),
            video_source,
            audio,
            last_mesh_scale: 100,
            needs_mesh_rebuild: false,
            show_help: false,
            video_width: args.width,
            video_height: args.height,
        }
    }

    fn handle_keyboard(&mut self, key: KeyCode, pressed: bool) {
        if !pressed {
            return;
        }

        // Debug: log all key presses
        log::info!("Key pressed: {:?}", key);

        // Help toggle
        if key == KeyCode::KeyH {
            self.show_help = !self.show_help;
            if self.show_help {
                self.print_help();
            }
            return;
        }

        let ko = &mut self.state.keyboard_offsets;

        match key {
            // Luma key
            KeyCode::KeyA => ko.az += 0.01,
            KeyCode::KeyZ => ko.az -= 0.01,

            // Z LFO
            KeyCode::KeyS => ko.sx += 0.0001,
            KeyCode::KeyX => ko.sx -= 0.0001,
            KeyCode::KeyD => ko.dc += 0.001,
            KeyCode::KeyC => ko.dc -= 0.001,
            KeyCode::KeyF => ko.fv += 0.001,
            KeyCode::KeyV => ko.fv -= 0.001,

            // X LFO
            KeyCode::KeyG => ko.gb += 0.001,
            KeyCode::KeyB => ko.gb -= 0.001,
            KeyCode::KeyH => ko.hn += 0.001,
            KeyCode::KeyN => ko.hn -= 0.001,
            KeyCode::KeyJ => ko.jm += 0.1,
            KeyCode::KeyM => ko.jm -= 0.1,

            // Y LFO
            KeyCode::KeyK => ko.kk += 0.001,
            KeyCode::Comma => ko.kk -= 0.001,
            KeyCode::KeyL => ko.ll += 0.001,
            KeyCode::Period => ko.ll -= 0.001,
            KeyCode::Semicolon => ko.ylfo_amp += 0.1,
            KeyCode::Slash => ko.ylfo_amp -= 0.1,

            // Center offset
            KeyCode::KeyT => ko.ty += 5.0,
            KeyCode::KeyY => ko.ty -= 5.0,
            KeyCode::KeyU => ko.ui += 5.0,
            KeyCode::KeyI => ko.ui -= 5.0,

            // Zoom
            KeyCode::KeyO => ko.op += 5.0,
            KeyCode::KeyP => ko.op -= 5.0,

            // Displacement
            KeyCode::KeyE => ko.er += 0.01,
            KeyCode::KeyR => ko.er -= 0.01,
            KeyCode::KeyQ => ko.qw += 0.01,
            KeyCode::KeyW => ko.qw -= 0.01,

            // Scale
            KeyCode::BracketRight => {
                ko.scale_key += 1;
                self.needs_mesh_rebuild = true;
            }
            KeyCode::BracketLeft => {
                ko.scale_key -= 1;
                self.needs_mesh_rebuild = true;
            }

            // Toggles
            KeyCode::Digit1 => self.state.luma_switch = !self.state.luma_switch,
            KeyCode::Digit2 => self.state.bright_switch = !self.state.bright_switch,
            KeyCode::Digit3 => self.state.invert = !self.state.invert,
            KeyCode::Digit5 => self.state.greyscale = !self.state.greyscale,

            // LFO shapes
            KeyCode::Digit6 => self.state.z_lfo_shape = (self.state.z_lfo_shape + 1) % 4,
            KeyCode::Digit7 => self.state.x_lfo_shape = (self.state.x_lfo_shape + 1) % 4,
            KeyCode::Digit8 => self.state.y_lfo_shape = (self.state.y_lfo_shape + 1) % 4,

            // Mesh types
            KeyCode::Digit9 => {
                self.state.mesh_type = mesh::MeshType::VerticalLines;
                self.needs_mesh_rebuild = true;
            }
            KeyCode::Digit0 => {
                self.state.mesh_type = mesh::MeshType::HorizontalLines;
                self.needs_mesh_rebuild = true;
            }
            KeyCode::Minus => {
                log::info!("Minus pressed - Triangles filled");
                self.state.mesh_type = mesh::MeshType::Triangles;
                self.needs_mesh_rebuild = true;
            }
            KeyCode::Equal => {
                log::info!("Equal pressed - Grid (wireframe)");
                self.state.mesh_type = mesh::MeshType::Grid;
                self.needs_mesh_rebuild = true;
            }

            // Audio sensitivity controls
            KeyCode::ArrowUp => {
                self.state.audio_sensitivity = (self.state.audio_sensitivity + 0.1).min(5.0);
                log::info!("Audio sensitivity: {:.1}", self.state.audio_sensitivity);
            }
            KeyCode::ArrowDown => {
                self.state.audio_sensitivity = (self.state.audio_sensitivity - 0.1).max(0.0);
                log::info!("Audio sensitivity: {:.1}", self.state.audio_sensitivity);
            }

            _ => {}
        }
    }

    fn print_help(&self) {
        println!("\n╔════════════════════════════════════════════════════════════════╗");
        println!("║              SPECTRAL MESH v5.0 - CONTROLS                     ║");
        println!("╠════════════════════════════════════════════════════════════════╣");
        println!("║ H        : Toggle this help                                    ║");
        println!("║ ESC      : Quit                                                ║");
        println!("╠════════════════════════════════════════════════════════════════╣");
        println!("║ MESH TYPE                                                      ║");
        println!("║ 9        : Vertical lines                                      ║");
        println!("║ 0        : Horizontal lines                                    ║");
        println!("║ -        : Triangles (filled)                                  ║");
        println!("║ =        : Triangles (wireframe)                               ║");
        println!("║ [ / ]    : Decrease / Increase grid density                    ║");
        println!("╠════════════════════════════════════════════════════════════════╣");
        println!("║ EFFECTS                                                        ║");
        println!("║ 1        : Toggle luma key mode                                ║");
        println!("║ 2        : Toggle brightness mode                              ║");
        println!("║ 3        : Toggle color inversion                              ║");
        println!("║ 5        : Toggle greyscale                                    ║");
        println!("║ A / Z    : Luma key level +/-                                  ║");
        println!("╠════════════════════════════════════════════════════════════════╣");
        println!("║ LFO SHAPES (cycle: sine -> square -> saw -> triangle)          ║");
        println!("║ 6        : Z LFO shape                                         ║");
        println!("║ 7        : X LFO shape                                         ║");
        println!("║ 8        : Y LFO shape                                         ║");
        println!("╠════════════════════════════════════════════════════════════════╣");
        println!("║ Z LFO (zoom/scale)                                             ║");
        println!("║ S / X    : Frequency +/-                                       ║");
        println!("║ D / C    : Phase +/-                                           ║");
        println!("║ F / V    : Amplitude +/-                                       ║");
        println!("╠════════════════════════════════════════════════════════════════╣");
        println!("║ X LFO (horizontal waves)                                       ║");
        println!("║ G / B    : Frequency +/-                                       ║");
        println!("║ H / N    : Phase +/-                                           ║");
        println!("║ J / M    : Amplitude +/-                                       ║");
        println!("╠════════════════════════════════════════════════════════════════╣");
        println!("║ Y LFO (vertical waves)                                         ║");
        println!("║ K / ,    : Frequency +/-                                       ║");
        println!("║ L / .    : Phase +/-                                           ║");
        println!("║ ; / /    : Amplitude +/-                                       ║");
        println!("╠════════════════════════════════════════════════════════════════╣");
        println!("║ DISPLACEMENT                                                   ║");
        println!("║ Q / W    : X displacement +/-                                  ║");
        println!("║ E / R    : Y displacement +/-                                  ║");
        println!("╠════════════════════════════════════════════════════════════════╣");
        println!("║ POSITION                                                       ║");
        println!("║ T / Y    : Center X +/-                                        ║");
        println!("║ U / I    : Center Y +/-                                        ║");
        println!("║ O / P    : Zoom +/-                                            ║");
        println!("╚════════════════════════════════════════════════════════════════╝");
        if self.audio.is_some() {
            println!("║ AUDIO    : Active (modulating displacement & LFO)             ║");
        } else {
            println!("║ AUDIO    : Disabled (use --audio to enable)                   ║");
        }
        println!();
    }

    fn update(&mut self) {
        // Process MIDI
        if let Some(ref midi) = self.midi {
            for cmd in midi.poll_all() {
                self.state.process_midi(cmd);
            }
        }

        // Update p_lock system
        self.state.p_lock.update();

        // Audio modulation - aesthetic effect: bass modulates displacement and LFO
        if let Some(ref mut audio) = self.audio {
            let sensitivity = self.state.audio_sensitivity;
            let bass = audio.bass() * sensitivity;
            let rms = audio.rms() * sensitivity;

            // Reduced amplitude for subtle global effect
            self.state.audio_mod_displacement = bass * 2.0;
            self.state.audio_mod_lfo = rms * 1.0;
            self.state.audio_mod_z = bass * 0.02;

            // Audio vibration effect - lines tremble with the music
            // Phase advances fast for vibration effect
            let phase_speed = 0.5 + bass * 1.5; // Faster base speed, accelerates with bass
            self.state.audio_wave_phase += phase_speed;

            // Amplitude pulses with bass - fast attack, slower decay
            let target_amp = bass * 0.08; // Vibration amplitude
            // Fast attack (0.4), slower decay (0.9) for punchy response
            if target_amp > self.state.audio_wave_amp {
                self.state.audio_wave_amp = self.state.audio_wave_amp * 0.6 + target_amp * 0.4;
            } else {
                self.state.audio_wave_amp = self.state.audio_wave_amp * 0.92 + target_amp * 0.08;
            }

            // Frequency not used for vibration but keep for potential future use
            self.state.audio_wave_freq = 10.0 + rms * 20.0;
        }

        // Calculate render params
        let params = self.state.calculate_render_params();

        // Update LFO phases - no wrapping to avoid discontinuities
        // Precision issues won't occur for hours of continuous use
        self.state.z_lfo_arg += params.z_lfo_arg;
        self.state.x_lfo_arg += params.x_lfo_arg;
        self.state.y_lfo_arg += params.y_lfo_arg;

        // Update noise textures
        self.noise_bank.update(
            self.state.x_lfo_arg,
            self.state.p_lock.get(4),
            self.state.y_lfo_arg,
            self.state.p_lock.get(5),
            self.state.z_lfo_arg,
            self.state.p_lock.get(3),
        );

        // Check if mesh needs rebuild
        let new_scale = params.scale.clamp(1, 127);
        if new_scale != self.last_mesh_scale || self.needs_mesh_rebuild {
            self.last_mesh_scale = new_scale;
            self.needs_mesh_rebuild = false;
            self.state.scale = new_scale;
        }
    }

    fn render(&mut self) {
        // Update video texture
        let frame = match &mut self.video_source {
            VideoSource::Camera(cam) => {
                cam.get_frame();
                cam.current_frame()
            }
            VideoSource::Dummy(dummy) => dummy.update(),
        };
        self.renderer.update_video_texture(frame, self.video_width, self.video_height);

        // Update noise textures
        self.renderer.update_noise_texture(0, self.noise_bank.x_noise.pixels(), NOISE_WIDTH, NOISE_HEIGHT);
        self.renderer.update_noise_texture(1, self.noise_bank.y_noise.pixels(), NOISE_WIDTH, NOISE_HEIGHT);
        self.renderer.update_noise_texture(2, self.noise_bank.z_noise.pixels(), NOISE_WIDTH, NOISE_HEIGHT);

        // Rebuild mesh if needed
        let mesh = match self.state.mesh_type {
            mesh::MeshType::Triangles => {
                Mesh::triangle_mesh(self.state.scale, self.video_width as f32, self.video_height as f32)
            }
            mesh::MeshType::HorizontalLines => {
                Mesh::horizontal_line_mesh(self.state.scale, self.video_width as f32, self.video_height as f32)
            }
            mesh::MeshType::VerticalLines => {
                Mesh::vertical_line_mesh(self.state.scale, self.video_width as f32, self.video_height as f32)
            }
            mesh::MeshType::Grid => {
                Mesh::grid_mesh(self.state.scale, self.video_width as f32, self.video_height as f32)
            }
        };
        self.renderer.update_mesh(&mesh);

        // Update uniforms
        self.renderer.update_uniforms(&self.state);

        // Render
        match self.renderer.render() {
            Ok(_) => {}
            Err(wgpu::SurfaceError::Lost) => self.renderer.resize(self.renderer.size),
            Err(wgpu::SurfaceError::OutOfMemory) => {
                log::error!("Out of memory");
                std::process::exit(1);
            }
            Err(e) => log::warn!("Render error: {:?}", e),
        }
    }

    fn resize(&mut self, size: winit::dpi::PhysicalSize<u32>) {
        self.renderer.resize(size);
    }
}

fn list_all_devices() {
    println!("\n=== MIDI INPUT DEVICES ===");
    if let Ok(midi_in) = midir::MidiInput::new("list") {
        let ports = midi_in.ports();
        if ports.is_empty() {
            println!("  No MIDI devices found");
        } else {
            for (i, port) in ports.iter().enumerate() {
                let name = midi_in.port_name(port).unwrap_or_else(|_| "Unknown".to_string());
                println!("  {}: {}", i, name);
            }
        }
    }

    println!("\n=== VIDEO INPUT DEVICES ===");
    #[cfg(feature = "camera")]
    {
        println!("  Available camera indices: 0-5");
        println!("  Use --video <index> to select");
        println!("  (Camera enumeration requires device access)");
    }
    #[cfg(not(feature = "camera"))]
    {
        println!("  Camera support not compiled");
    }

    println!("\n=== AUDIO INPUT DEVICES ===");
    let audio_devices = audio::list_audio_devices();
    if audio_devices.is_empty() {
        println!("  No audio devices found");
    } else {
        for (i, name) in audio_devices.iter().enumerate() {
            println!("  {}: {}", i, name);
        }
    }

    println!();
}

fn main() {
    env_logger::init();

    let args = Args::parse();

    if args.list_devices {
        list_all_devices();
        return;
    }

    log::info!("Starting Spectral Mesh v5.0");
    log::info!("Rust/wgpu port - Cross-platform (macOS/Linux/Raspberry Pi)");
    log::info!("Video: {}x{}, MIDI port: {}", args.width, args.height, args.midi);

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    let window = std::sync::Arc::new(
        WindowBuilder::new()
            .with_title("Spectral Mesh v5.0 (Rust/wgpu)")
            .with_inner_size(winit::dpi::LogicalSize::new(args.window_width, args.window_height))
            .build(&event_loop)
            .unwrap(),
    );

    let renderer = pollster::block_on(Renderer::new(window.clone()));
    let mut app = App::new(renderer, &args);

    event_loop
        .run(move |event, elwt| {
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    WindowEvent::Resized(physical_size) => {
                        app.resize(physical_size);
                    }
                    WindowEvent::KeyboardInput {
                        event:
                            KeyEvent {
                                physical_key: PhysicalKey::Code(key),
                                state,
                                ..
                            },
                        ..
                    } => {
                        // ESC disabled - use Ctrl+C or close window to quit
                        app.handle_keyboard(key, state == ElementState::Pressed);
                    }
                    WindowEvent::RedrawRequested => {
                        app.update();
                        app.render();
                    }
                    _ => {}
                },
                Event::AboutToWait => {
                    window.request_redraw();
                }
                _ => {}
            }
        })
        .unwrap();
}
