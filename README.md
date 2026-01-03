# Spectral Mesh RS

A Rust/wgpu port of [Spectral Mesh](https://github.com/ex-zee-ex/spectral_mesh) by **Andrei Jay** ([@ex-zee-ex](https://github.com/ex-zee-ex)).

This is a complete rewrite in Rust targeting cross-platform support (macOS, Linux, Raspberry Pi 4) while preserving the original's visual aesthetic and MIDI control philosophy.

## What is Spectral Mesh?

Spectral Mesh is a real-time video mesh distortion tool for live visual performances. It captures video input, applies it to a deformable mesh, and distorts it using brightness-based displacement and multiple LFO modulation sources. Perfect for VJing, live concerts, and experimental video art.

## Differences from the Original

### Technical Stack

| Component | Original (C++) | This Port (Rust) |
|-----------|----------------|------------------|
| Language | C++ | Rust |
| Framework | openFrameworks | Native Rust |
| Graphics API | OpenGL | wgpu (Metal/Vulkan/DX12) |
| Shaders | GLSL | WGSL |
| Video Capture | ofVideoGrabber | nokhwa |
| Audio Input | ofSoundStream | cpal |
| MIDI | ofxMidi | midir |
| GUI | openFrameworks GUI | None (CLI only) |

### Platform Support

| Platform | Original | This Port |
|----------|----------|-----------|
| macOS | Yes | Yes (Metal backend) |
| Windows | Limited | Yes (DX12/Vulkan backend) |
| Linux | Limited | Yes (Vulkan backend) |
| Raspberry Pi 4 | No | Yes (Vulkan backend) |

### Functional Differences

**Preserved from original:**
- All mesh types (triangles, horizontal lines, vertical lines, grid)
- 3 LFO channels (X, Y, Z) with full modulation chain
- 4 waveforms per LFO (sine, square, triangle, noise)
- Ring modulation and phase modulation between LFOs
- Brightness-based displacement
- Luma keying
- Color inversion and greyscale modes
- MIDI CC control with same mapping
- P-Lock parameter recording system

**Changes in this port:**
- Saw wave replaced with triangle wave (smoother, no visual discontinuity)
- No GUI - keyboard and MIDI control only
- Audio reactivity simplified (bass-driven displacement)
- Configurable video resolution via command line

**Not yet implemented:**
- Syphon/Spout output
- OSC control
- Preset save/load to files
- Some advanced MIDI mappings (rotation, global displacement)

## Features

- **Real-time video capture** with configurable resolution
- **Multiple mesh types**: Triangles, Horizontal Lines, Vertical Lines, Grid (wireframe)
- **3 LFO channels** (X, Y, Z) with:
  - 4 waveforms: Sine, Square, Triangle, Noise
  - Ring modulation between channels
  - Phase modulation between channels
  - Spatial frequency control
- **Audio reactivity**:
  - Mesh displacement responds to bass frequencies
  - LFO modulation driven by audio RMS
  - Adjustable sensitivity (0.0 to 5.0)
- **MIDI control**: Full parameter control via MIDI CC
- **P-Lock system**: Parameter recording and playback (Elektron-style)
- **Visual effects**: Luma key, color inversion, greyscale, brightness modes

## Building

### Requirements

- Rust 1.70+ (install via [rustup](https://rustup.rs/))
- On Linux: `libudev-dev`, `libv4l-dev`, `libasound2-dev`

### Build

```bash
cargo build --release
```

The binary will be at `./target/release/spectral_mesh`

## Usage

```bash
# Basic usage (uses default camera and audio)
./target/release/spectral_mesh

# List available devices
./target/release/spectral_mesh --list-devices

# Specify devices
./target/release/spectral_mesh --video 0 --midi 1 --audio 3

# Custom resolution (lower = faster, useful for Raspberry Pi)
./target/release/spectral_mesh --width 640 --height 360

# Custom window size
./target/release/spectral_mesh --window-width 1920 --window-height 1080
```

## Keyboard Controls

| Key | Function |
|-----|----------|
| **Mesh Type** | |
| `9` | Vertical lines |
| `0` | Horizontal lines |
| `-` | Triangles (filled) |
| `=` | Grid (wireframe) |
| `[` / `]` | Decrease / Increase grid density |
| **Effects** | |
| `1` | Toggle luma key mode |
| `2` | Toggle brightness invert |
| `3` | Toggle color inversion |
| `5` | Toggle greyscale |
| `A` / `Z` | Luma key level +/- |
| **LFO Shapes** (cycle: sine → square → triangle → noise) | |
| `6` | Cycle Z LFO shape |
| `7` | Cycle X LFO shape |
| `8` | Cycle Y LFO shape |
| **Z LFO (zoom/scale)** | |
| `S` / `X` | Frequency +/- |
| `D` / `C` | Phase +/- |
| `F` / `V` | Amplitude +/- |
| **X LFO (horizontal waves)** | |
| `G` / `B` | Frequency +/- |
| `H` / `N` | Phase +/- |
| `J` / `M` | Amplitude +/- |
| **Y LFO (vertical waves)** | |
| `K` / `,` | Frequency +/- |
| `L` / `.` | Phase +/- |
| `;` / `/` | Amplitude +/- |
| **Displacement** | |
| `Q` / `W` | X displacement +/- |
| `E` / `R` | Y displacement +/- |
| **Position** | |
| `T` / `Y` | Center X +/- |
| `U` / `I` | Center Y +/- |
| `O` / `P` | Zoom +/- |
| **Audio Reactivity** | |
| `Arrow Up` | Increase audio sensitivity (+0.1, max 5.0) |
| `Arrow Down` | Decrease audio sensitivity (-0.1, min 0.0) |
| **Other** | |
| `H` | Show help in terminal |
| Close window or `Ctrl+C` | Quit |

## MIDI Mapping

Compatible with the original Spectral Mesh MIDI mapping (Faderfox MX12 layout). Main CC assignments:

| CC | Function |
|----|----------|
| 1 | Luma key level |
| 2 | X displacement |
| 3 | Y displacement |
| 4 | Z LFO frequency |
| 5 | X LFO frequency |
| 6 | Y LFO frequency |
| 7 | Zoom |
| 8 | Grid scale |
| ... | See source code for full mapping |

## Performance Tips

- Lower resolution (`--width 640 --height 360`) for better performance on slower hardware
- Raspberry Pi 4: Use 640x360 or 480x270 for smooth 30fps
- Reduce grid density with `[` key if frame rate drops

## Credits

**Original concept and design**: [Andrei Jay](https://github.com/ex-zee-ex)

The original Spectral Mesh is an openFrameworks/GLSL application that pioneered this unique approach to live video mesh manipulation. This Rust port aims to make it accessible on more platforms while honoring the original vision.

## License

Licensing pending discussion with the original author. Please contact before redistribution.

## Contributing

Issues and pull requests welcome! Please ensure any contributions maintain compatibility with Raspberry Pi 4 as a target platform.
