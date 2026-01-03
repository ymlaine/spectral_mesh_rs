// Spectral Mesh - Displacement Shader (WGSL)
// With audio-driven wave modulation for undulating mesh lines

struct Uniforms {
    mvp: mat4x4<f32>,
    xy: vec2<f32>,              // displacement multiplier
    xy_offset: vec2<f32>,       // center offset
    x_lfo_arg: f32,
    x_lfo_amp: f32,
    x_lfo_other: f32,
    y_lfo_arg: f32,
    y_lfo_amp: f32,
    y_lfo_other: f32,
    z_lfo_arg: f32,
    z_lfo_amp: f32,
    z_lfo_other: f32,
    luma_key_level: f32,
    invert_switch: f32,
    b_w_switch: f32,
    bright_switch: i32,
    x_lfo_shape: i32,
    y_lfo_shape: i32,
    z_lfo_shape: i32,
    x_ringmod_switch: i32,
    y_ringmod_switch: i32,
    z_ringmod_switch: i32,
    x_phasemod_switch: i32,
    y_phasemod_switch: i32,
    z_phasemod_switch: i32,
    luma_switch: i32,
    width: i32,
    height: i32,
    audio_displacement: f32,
    audio_z: f32,
    audio_wave_phase: f32,      // wave phase for line undulation
    audio_wave_amp: f32,        // wave amplitude from bass
    audio_wave_freq: f32,       // wave frequency from audio energy
    _pad0: f32,
    _pad1: f32,
    _pad2: f32,
    _pad3: f32,
    _pad4: f32,
    _pad5: f32,
}

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var video_texture: texture_2d<f32>;
@group(0) @binding(2) var video_sampler: sampler;
@group(0) @binding(3) var x_noise_texture: texture_2d<f32>;
@group(0) @binding(4) var y_noise_texture: texture_2d<f32>;
@group(0) @binding(5) var z_noise_texture: texture_2d<f32>;
@group(0) @binding(6) var noise_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coord: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
}

const TWO_PI: f32 = 6.283185307;

// Oscillator function with soft saw (triangle wave instead of hard saw)
fn oscillate(theta: f32, shape: i32, tex_coord: vec2<f32>) -> f32 {
    if shape == 0 {
        // Sine - smooth
        return sin(theta);
    } else if shape == 1 {
        // Square - hard edges but no sweeping discontinuity
        return sign(sin(theta));
    } else if shape == 2 {
        // Triangle wave (soft saw) - continuous, no discontinuity
        // Goes from -1 to 1 to -1 smoothly
        let t = fract(theta / TWO_PI);
        return 4.0 * abs(t - 0.5) - 1.0;
    } else {
        // Noise - sample from noise texture
        let noise_val = textureSampleLevel(x_noise_texture, noise_sampler, tex_coord * 0.5, 0.0).r;
        return 2.0 * (noise_val - 0.5);
    }
}

// Audio-driven vibration effect - disabled for now
fn audio_vibration(tex_coord: vec2<f32>) -> vec2<f32> {
    return vec2<f32>(0.0, 0.0);
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.tex_coord = in.tex_coord;

    // Transform to clip space first (like original)
    var new_position = uniforms.mvp * vec4<f32>(in.position, 1.0);

    // Sample video and calculate brightness
    let color = textureSampleLevel(video_texture, video_sampler, in.tex_coord, 0.0);
    var bright = 0.33 * color.r + 0.5 * color.g + 0.16 * color.b;

    // Logarithmic brightness boost (from original)
    bright = 2.0 * log(1.0 + bright);

    // Invert brightness if bright_switch is on
    if uniforms.bright_switch == 1 {
        bright = 1.0 - bright;
    }

    // Add center offset
    new_position.x = new_position.x + uniforms.xy_offset.x;
    new_position.y = new_position.y + uniforms.xy_offset.y;

    // Full LFO chain restored

    // Initial X LFO for modulation chain
    let x_lfo_initial = uniforms.x_lfo_amp * oscillate(
        uniforms.x_lfo_arg + new_position.y * uniforms.x_lfo_other,
        uniforms.x_lfo_shape,
        in.tex_coord
    );

    // Y LFO with optional ring/phase modulation from X
    var y_lfo = (uniforms.y_lfo_amp + f32(uniforms.y_ringmod_switch) * 0.01 * x_lfo_initial) * oscillate(
        uniforms.y_lfo_arg + new_position.x * uniforms.y_lfo_other + f32(uniforms.y_phasemod_switch) * 0.01 * x_lfo_initial,
        uniforms.y_lfo_shape,
        in.tex_coord
    );

    // Z LFO (affects scale/zoom) with optional modulation from Y
    let z_lfo_amp_mod = uniforms.z_lfo_amp + f32(uniforms.z_ringmod_switch) * 0.0025 * y_lfo + uniforms.audio_z;
    let z_lfo_freq = uniforms.z_lfo_arg + uniforms.z_lfo_other * distance(
        abs(new_position.xy),
        vec2<f32>(uniforms.xy_offset.x / 2.0, uniforms.xy_offset.y / 2.0)
    ) + f32(uniforms.z_phasemod_switch) * y_lfo;
    let z_lfo = z_lfo_amp_mod * oscillate(z_lfo_freq, uniforms.z_lfo_shape, in.tex_coord);

    // Apply Z LFO as scale
    new_position.x = new_position.x * (1.0 - z_lfo);
    new_position.y = new_position.y * (1.0 - z_lfo);

    // X LFO with optional ring/phase modulation from Z
    let x_lfo_amp_mod = uniforms.x_lfo_amp + f32(uniforms.x_ringmod_switch) * 1000.0 * z_lfo;
    let x_lfo_freq = uniforms.x_lfo_arg + new_position.y * uniforms.x_lfo_other + f32(uniforms.x_phasemod_switch) * 10.0 * z_lfo;
    let x_lfo = x_lfo_amp_mod * oscillate(x_lfo_freq, uniforms.x_lfo_shape, in.tex_coord);

    // Apply X displacement: brightness * xy + x_lfo + audio
    new_position.x = new_position.x + (uniforms.xy.x + uniforms.audio_displacement) * bright + x_lfo;

    // Y LFO recalculated with optional ring/phase modulation from X
    let y_lfo_amp_mod = uniforms.y_lfo_amp + f32(uniforms.y_ringmod_switch) * x_lfo;
    let y_lfo_freq = uniforms.y_lfo_arg + new_position.x * uniforms.y_lfo_other + f32(uniforms.y_phasemod_switch) * 0.01 * x_lfo;
    y_lfo = y_lfo_amp_mod * oscillate(y_lfo_freq, uniforms.y_lfo_shape, in.tex_coord);

    // Apply Y displacement: brightness * xy + y_lfo + audio
    new_position.y = new_position.y + (uniforms.xy.y + uniforms.audio_displacement) * bright + y_lfo;

    // Apply audio vibration effect - lines tremble with the music
    let vib_disp = audio_vibration(in.tex_coord);
    new_position.x = new_position.x + vib_disp.x;
    new_position.y = new_position.y + vib_disp.y;

    // Remove center offset
    new_position.x = new_position.x - uniforms.xy_offset.x;
    new_position.y = new_position.y - uniforms.xy_offset.y;

    out.clip_position = new_position;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color = textureSample(video_texture, video_sampler, in.tex_coord);
    let bright = 0.33 * color.r + 0.5 * color.g + 0.16 * color.b;

    // Greyscale blend (matches original: b_w_switch * grey + (1-b_w_switch) * color)
    let grey = vec4<f32>(bright, bright, bright, color.a);
    color = uniforms.b_w_switch * grey + (1.0 - uniforms.b_w_switch) * color;

    // Invert (matches original)
    color = vec4<f32>(
        uniforms.invert_switch * (1.0 - color.r) + (1.0 - uniforms.invert_switch) * color.r,
        uniforms.invert_switch * (1.0 - color.g) + (1.0 - uniforms.invert_switch) * color.g,
        uniforms.invert_switch * (1.0 - color.b) + (1.0 - uniforms.invert_switch) * color.b,
        color.a
    );

    // Luma key (matches original)
    if uniforms.luma_switch == 0 && bright < uniforms.luma_key_level {
        color.a = 0.0;
    }
    if uniforms.luma_switch == 1 && bright > uniforms.luma_key_level {
        color.a = 0.0;
    }

    return color;
}
