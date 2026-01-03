use midir::{Ignore, MidiInput, MidiInputConnection};
use std::sync::mpsc::{channel, Receiver, Sender};

const MIDI_MAGIC: f32 = 63.50;
const CONTROL_THRESHOLD: f32 = 0.04;

#[derive(Debug, Clone)]
pub enum MidiCommand {
    // Continuous controls (knobs/faders)
    LumaKeyLevel(f32),        // CC 16
    DisplaceX(f32),           // CC 17
    DisplaceY(f32),           // CC 18
    ZFrequency(f32),          // CC 19
    XFrequency(f32),          // CC 20
    YFrequency(f32),          // CC 21
    Zoom(f32),                // CC 22
    Scale(f32),               // CC 23

    CenterX(f32),             // CC 120
    CenterY(f32),             // CC 121
    ZLfoArg(f32),             // CC 122
    ZLfoAmp(f32),             // CC 123
    XLfoArg(f32),             // CC 124
    XLfoAmp(f32),             // CC 125
    YLfoArg(f32),             // CC 126
    YLfoAmp(f32),             // CC 127

    // Toggle switches
    RecordStart,              // CC 60 value 127
    RecordStop,               // CC 60 value 0
    Reset,                    // CC 58 value 127

    // LFO shapes (0=sine, 1=square, 2=saw, 3=noise)
    ZLfoShape(i32),
    XLfoShape(i32),
    YLfoShape(i32),

    // Modulation switches
    ZRingMod(bool),
    XRingMod(bool),
    YRingMod(bool),
    ZPhaseMod(bool),
    XPhaseMod(bool),
    YPhaseMod(bool),

    // Mesh type
    SetTriangleMesh,
    SetHorizontalLines,
    SetVerticalLines,
    SetWireframe,

    // Visual effects
    Greyscale(bool),
    Invert(bool),
    BrightSwitch(bool),
    StrokeWeight(f32),

    // Mode switches
    GlobalXDisplace(bool),
    CenterXDisplace(bool),
    RotateX(f32),
    RotateY(f32),
    RotateZ(f32),
    GlobalYDisplace(bool),
    CenterYDisplace(bool),

    // Frequency zero switches
    ZFreqZero(bool),
    XFreqZero(bool),
    YFreqZero(bool),
}

pub struct MidiHandler {
    #[allow(dead_code)]
    connection: Option<MidiInputConnection<()>>,
    receiver: Receiver<MidiCommand>,
}

impl MidiHandler {
    pub fn new(port_index: usize) -> Result<Self, String> {
        let midi_in = MidiInput::new("spectral_mesh")
            .map_err(|e| format!("Failed to create MIDI input: {}", e))?;

        let in_ports = midi_in.ports();
        if in_ports.is_empty() {
            log::warn!("No MIDI input ports available");
            let (_, receiver) = channel();
            return Ok(Self {
                connection: None,
                receiver,
            });
        }

        // List available ports
        for (i, port) in in_ports.iter().enumerate() {
            let name = midi_in.port_name(port).unwrap_or_default();
            log::info!("MIDI port {}: {}", i, name);
        }

        let port = in_ports.get(port_index).ok_or_else(|| {
            format!(
                "MIDI port {} not available (found {} ports)",
                port_index,
                in_ports.len()
            )
        })?;

        let port_name = midi_in.port_name(port).unwrap_or_default();
        log::info!("Connecting to MIDI port: {}", port_name);

        let (sender, receiver) = channel::<MidiCommand>();

        let mut midi_in = MidiInput::new("spectral_mesh_handler")
            .map_err(|e| format!("Failed to create MIDI handler: {}", e))?;
        midi_in.ignore(Ignore::None);

        let connection = midi_in
            .connect(
                port,
                "spectral_mesh_input",
                move |_stamp, message, _| {
                    if message.len() >= 3 {
                        Self::process_message(message, &sender);
                    }
                },
                (),
            )
            .map_err(|e| format!("Failed to connect to MIDI port: {}", e))?;

        Ok(Self {
            connection: Some(connection),
            receiver,
        })
    }

    fn process_message(message: &[u8], sender: &Sender<MidiCommand>) {
        let status = message[0] & 0xF0;
        let control = message[1];
        let value = message[2];

        // Control Change messages
        if status == 0xB0 {
            let normalized = value as f32 / 127.0;
            let bipolar = (value as f32 - MIDI_MAGIC) / MIDI_MAGIC;

            let cmd = match control {
                // Main continuous controls
                16 => Some(MidiCommand::LumaKeyLevel(normalized)),
                17 => Some(MidiCommand::DisplaceX(bipolar)),
                18 => Some(MidiCommand::DisplaceY(bipolar)),
                19 => Some(MidiCommand::ZFrequency(normalized)),
                20 => Some(MidiCommand::XFrequency(bipolar)),
                21 => Some(MidiCommand::YFrequency(bipolar)),
                22 => Some(MidiCommand::Zoom(bipolar)),
                23 => Some(MidiCommand::Scale(normalized)),

                // Center/offset controls
                120 => Some(MidiCommand::CenterX(bipolar)),
                121 => Some(MidiCommand::CenterY(bipolar)),
                122 => Some(MidiCommand::ZLfoArg(bipolar * 0.1)),
                123 => Some(MidiCommand::ZLfoAmp(bipolar)),
                124 => Some(MidiCommand::XLfoArg(bipolar * 0.1)),
                125 => Some(MidiCommand::XLfoAmp(bipolar)),
                126 => Some(MidiCommand::YLfoArg(bipolar * 0.1)),
                127 => Some(MidiCommand::YLfoAmp(bipolar)),

                // Record/reset
                60 => {
                    if value == 127 {
                        Some(MidiCommand::RecordStart)
                    } else {
                        Some(MidiCommand::RecordStop)
                    }
                }
                58 => {
                    if value == 127 {
                        Some(MidiCommand::Reset)
                    } else {
                        None
                    }
                }

                // Z LFO shapes
                35 => Some(MidiCommand::ZLfoShape(if value == 127 { 1 } else { 0 })),
                51 => Some(MidiCommand::ZLfoShape(if value == 127 { 2 } else { 0 })),
                67 => Some(MidiCommand::ZLfoShape(if value == 127 { 3 } else { 0 })),

                // X LFO shapes
                37 => Some(MidiCommand::XLfoShape(if value == 127 { 1 } else { 0 })),
                53 => Some(MidiCommand::XLfoShape(if value == 127 { 2 } else { 0 })),
                69 => Some(MidiCommand::XLfoShape(if value == 127 { 3 } else { 0 })),

                // Y LFO shapes
                39 => Some(MidiCommand::YLfoShape(if value == 127 { 1 } else { 0 })),
                55 => Some(MidiCommand::YLfoShape(if value == 127 { 2 } else { 0 })),
                71 => Some(MidiCommand::YLfoShape(if value == 127 { 3 } else { 0 })),

                // Ring/phase modulation
                34 => Some(MidiCommand::ZFreqZero(value == 127)),
                50 => Some(MidiCommand::ZRingMod(value == 127)),
                66 => Some(MidiCommand::ZPhaseMod(value == 127)),
                36 => Some(MidiCommand::XFreqZero(value == 127)),
                52 => Some(MidiCommand::XRingMod(value == 127)),
                68 => Some(MidiCommand::XPhaseMod(value == 127)),
                38 => Some(MidiCommand::YFreqZero(value == 127)),
                54 => Some(MidiCommand::YRingMod(value == 127)),
                70 => Some(MidiCommand::YPhaseMod(value == 127)),

                // Mesh types
                41 => {
                    if value == 127 {
                        Some(MidiCommand::SetWireframe)
                    } else {
                        None
                    }
                }
                42 => {
                    if value == 127 {
                        Some(MidiCommand::SetVerticalLines)
                    } else {
                        None
                    }
                }
                43 => {
                    if value == 127 {
                        Some(MidiCommand::SetTriangleMesh)
                    } else {
                        None
                    }
                }
                44 => {
                    if value == 127 {
                        Some(MidiCommand::SetHorizontalLines)
                    } else {
                        None
                    }
                }

                // Visual effects
                46 => Some(MidiCommand::Greyscale(value == 127)),
                59 => Some(MidiCommand::Invert(value == 127)),
                61 => Some(MidiCommand::BrightSwitch(value == 127)),
                45 => Some(MidiCommand::StrokeWeight(normalized * 5.0)),

                _ => None,
            };

            if let Some(cmd) = cmd {
                let _ = sender.send(cmd);
            }
        }
    }

    pub fn poll(&self) -> Option<MidiCommand> {
        self.receiver.try_recv().ok()
    }

    pub fn poll_all(&self) -> Vec<MidiCommand> {
        let mut commands = Vec::new();
        while let Ok(cmd) = self.receiver.try_recv() {
            commands.push(cmd);
        }
        commands
    }
}
