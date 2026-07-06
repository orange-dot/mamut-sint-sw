//! MIDI byte encoder for `mamut-seq`.
//!
//! The workspace has no MIDI *output* path: `mamut-runtime` only decodes
//! incoming messages (`audio_runtime/midi_parse.rs`). This module is the
//! inverse — it turns typed gestures into channel-voice status bytes the Mamut
//! receiver understands. Channels are 1-based in Mamut's world; the low nibble
//! carried on the wire is `channel - 1`.

/// Controller number for the mod wheel (honoured by the Mamut receiver).
pub const CC_MOD_WHEEL: u8 = 1;
/// Controller number for the sustain pedal (honoured by the Mamut receiver).
pub const CC_SUSTAIN: u8 = 64;
/// Channel-mode "all sound off" (ignored by Mamut, sent for MIDI citizenship).
pub const CC_ALL_SOUND_OFF: u8 = 120;
/// Channel-mode "reset all controllers" (ignored by Mamut).
pub const CC_RESET_ALL_CONTROLLERS: u8 = 121;
/// Channel-mode "all notes off" (ignored by Mamut).
pub const CC_ALL_NOTES_OFF: u8 = 123;

/// 14-bit pitch-bend centre (no bend).
pub const PITCH_BEND_CENTER: u16 = 0x2000;
/// Largest 14-bit pitch-bend value.
pub const PITCH_BEND_MAX: u16 = 0x3FFF;

/// A MIDI channel in the `1..=16` range used across the rig.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct MidiChannel(u8);

impl MidiChannel {
    /// Build a channel, rejecting anything outside `1..=16`.
    pub fn new(channel: u8) -> Option<Self> {
        if (1..=16).contains(&channel) {
            Some(Self(channel))
        } else {
            None
        }
    }

    /// The 1-based channel number.
    pub fn number(self) -> u8 {
        self.0
    }

    /// The low-nibble value carried on the wire (`channel - 1`).
    fn nibble(self) -> u8 {
        self.0 - 1
    }
}

/// Mask a value to the 7-bit MIDI data range `0..=127`.
pub fn data7(value: u8) -> u8 {
    value & 0x7F
}

/// Convert a normalized `0.0..=1.0` value to a 7-bit MIDI data byte.
pub fn clamp7(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 127.0).round() as u8
}

/// `Note On` (status `0x90`).
pub fn note_on(channel: MidiChannel, note: u8, velocity: u8) -> [u8; 3] {
    [0x90 | channel.nibble(), data7(note), data7(velocity)]
}

/// `Note Off` (status `0x80`, velocity `0`).
pub fn note_off(channel: MidiChannel, note: u8) -> [u8; 3] {
    [0x80 | channel.nibble(), data7(note), 0]
}

/// `Control Change` (status `0xB0`).
pub fn control_change(channel: MidiChannel, controller: u8, value: u8) -> [u8; 3] {
    [0xB0 | channel.nibble(), data7(controller), data7(value)]
}

/// `Channel Pressure` / channel aftertouch (status `0xD0`).
pub fn channel_aftertouch(channel: MidiChannel, pressure: u8) -> [u8; 2] {
    [0xD0 | channel.nibble(), data7(pressure)]
}

/// `Program Change` (status `0xC0`).
pub fn program_change(channel: MidiChannel, program: u8) -> [u8; 2] {
    [0xC0 | channel.nibble(), data7(program)]
}

/// `Pitch Bend` (status `0xE0`), 14-bit little-endian across two 7-bit bytes.
pub fn pitch_bend(channel: MidiChannel, value14: u16) -> [u8; 3] {
    let clamped = value14.min(PITCH_BEND_MAX);
    let lsb = (clamped & 0x7F) as u8;
    let msb = ((clamped >> 7) & 0x7F) as u8;
    [0xE0 | channel.nibble(), lsb, msb]
}

/// `Pitch Bend` from a normalized `-1.0..=1.0` value (`0.0` = centre).
pub fn pitch_bend_norm(channel: MidiChannel, normalized: f32) -> [u8; 3] {
    let clamped = normalized.clamp(-1.0, 1.0);
    let value = if clamped >= 0.0 {
        f32::from(PITCH_BEND_CENTER) + clamped * f32::from(PITCH_BEND_MAX - PITCH_BEND_CENTER)
    } else {
        f32::from(PITCH_BEND_CENTER) + clamped * f32::from(PITCH_BEND_CENTER)
    };
    let value14 = value.round().clamp(0.0, f32::from(PITCH_BEND_MAX)) as u16;
    pitch_bend(channel, value14)
}
