//! Live-mode state, keymap, and MIDI effects.
//!
//! Honest terminal constraint: plain terminals deliver no key-release events, so
//! the note model is a configurable gate length plus a latch/toggle mode. Where
//! the terminal supports the kitty keyboard protocol (`crossterm`
//! keyboard-enhancement flags), true press/release is used instead. The active
//! mode is shown in the UI.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::midi::{self, MidiChannel};
use crate::port::VirtualPort;
use crate::profile::Profile;
use crate::scenario::AbsoluteEvent;

const VELOCITY: u8 = 100;
const CC_STEP: f32 = 0.05;
const BEND_STEP: f32 = 0.10;
/// Channel-aftertouch ramp rate (normalized units per second).
const AFTERTOUCH_RATE: f32 = 3.0;
const OCTAVE_MIN: i32 = 0;
const OCTAVE_MAX: i32 = 9;

/// How note keys behave, given the terminal's key-release capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteMode {
    /// True press/release (requires kitty keyboard-enhancement).
    PressRelease,
    /// Keypress triggers a note released after a fixed gate length.
    Gate,
    /// Keypress toggles the note on/off.
    Latch,
}

impl NoteMode {
    pub fn label(self) -> &'static str {
        match self {
            NoteMode::PressRelease => "press/release",
            NoteMode::Gate => "gate",
            NoteMode::Latch => "latch",
        }
    }
}

/// A MIDI message scheduled to fire at a deadline (gate note-offs, fired
/// scenario events).
struct Scheduled {
    at: Instant,
    bytes: Vec<u8>,
    clears_note: Option<u8>,
}

/// A preloaded scenario that `f` fires into the live stream.
pub struct FiredScenario {
    pub name: String,
    pub events: Vec<AbsoluteEvent>,
}

/// Semantic actions produced by non-note keys.
enum Action {
    OctaveUp,
    OctaveDown,
    SustainToggle,
    AftertouchRampUp,
    AftertouchRampDown,
    AftertouchHold,
    CcNext,
    CcPrev,
    CcInc,
    CcDec,
    CcSetMax,
    CcSetMin,
    PitchUp,
    PitchDown,
    PitchCenter,
    Program(u8),
    Panic,
    Fire,
    NoteModeCycle,
}

pub struct LiveState {
    channel: MidiChannel,
    port_name: String,
    octave: i32,
    sustain: bool,
    program: Option<u8>,
    bend: f32,
    aftertouch: f32,
    aftertouch_target: f32,
    aftertouch_ramping: bool,
    last_aftertouch_sent: u8,
    held: BTreeSet<u8>,
    scheduled: Vec<Scheduled>,
    note_mode: NoteMode,
    release_available: bool,
    gate: Duration,
    controls: Vec<(String, u8)>,
    cc_values: Vec<f32>,
    cc_index: usize,
    fired: Option<FiredScenario>,
    log: String,
    last_tick: Instant,
    dirty: bool,
}

impl LiveState {
    pub fn new(
        channel: MidiChannel,
        port_name: &str,
        profile: &Profile,
        gate_ms: u64,
        release_available: bool,
        fired: Option<FiredScenario>,
        now: Instant,
    ) -> Self {
        let controls: Vec<(String, u8)> = profile.controls().to_vec();
        let cc_values = vec![0.0; controls.len()];
        let note_mode = if release_available {
            NoteMode::PressRelease
        } else {
            NoteMode::Gate
        };
        Self {
            channel,
            port_name: port_name.to_string(),
            octave: 5,
            sustain: false,
            program: None,
            bend: 0.0,
            aftertouch: 0.0,
            aftertouch_target: 0.0,
            aftertouch_ramping: false,
            last_aftertouch_sent: 0,
            held: BTreeSet::new(),
            scheduled: Vec::new(),
            note_mode,
            release_available,
            gate: Duration::from_millis(gate_ms),
            controls,
            cc_values,
            cc_index: 0,
            fired,
            log: "ready".to_string(),
            last_tick: now,
            dirty: true,
        }
    }

    // ---- read accessors for the renderer ----
    pub fn port_name(&self) -> &str {
        &self.port_name
    }
    pub fn channel(&self) -> u8 {
        self.channel.number()
    }
    pub fn octave(&self) -> i32 {
        self.octave
    }
    pub fn base_note(&self) -> u8 {
        self.note_of(0)
    }
    pub fn sustain(&self) -> bool {
        self.sustain
    }
    pub fn program(&self) -> Option<u8> {
        self.program
    }
    pub fn bend(&self) -> f32 {
        self.bend
    }
    pub fn aftertouch(&self) -> f32 {
        self.aftertouch
    }
    pub fn note_mode(&self) -> NoteMode {
        self.note_mode
    }
    pub fn release_available(&self) -> bool {
        self.release_available
    }
    pub fn gate_ms(&self) -> u128 {
        self.gate.as_millis()
    }
    pub fn held_count(&self) -> usize {
        self.held.len()
    }
    pub fn selected_control(&self) -> Option<(&str, f32)> {
        self.controls
            .get(self.cc_index)
            .map(|(name, _)| (name.as_str(), self.cc_values[self.cc_index]))
    }
    pub fn has_fire_scenario(&self) -> bool {
        self.fired.is_some()
    }
    pub fn log(&self) -> &str {
        &self.log
    }
    pub fn take_dirty(&mut self) -> bool {
        std::mem::replace(&mut self.dirty, false)
    }

    /// Advance time-based state: the aftertouch ramp and any scheduled messages.
    pub fn advance(&mut self, port: &mut VirtualPort, now: Instant) -> Result<()> {
        let dt = now.saturating_duration_since(self.last_tick).as_secs_f32();
        self.last_tick = now;

        if self.aftertouch_ramping {
            let step = AFTERTOUCH_RATE * dt;
            if (self.aftertouch - self.aftertouch_target).abs() <= step {
                self.aftertouch = self.aftertouch_target;
                self.aftertouch_ramping = false;
            } else if self.aftertouch < self.aftertouch_target {
                self.aftertouch += step;
            } else {
                self.aftertouch -= step;
            }
            let value = midi::clamp7(self.aftertouch);
            if value != self.last_aftertouch_sent {
                port.send_tracked(&midi::channel_aftertouch(self.channel, value))?;
                self.last_aftertouch_sent = value;
            }
            self.dirty = true;
        }

        let mut index = 0;
        while index < self.scheduled.len() {
            if self.scheduled[index].at <= now {
                let scheduled = self.scheduled.remove(index);
                port.send_tracked(&scheduled.bytes)?;
                if let Some(note) = scheduled.clears_note {
                    self.held.remove(&note);
                }
                self.dirty = true;
            } else {
                index += 1;
            }
        }
        Ok(())
    }

    /// Handle one key event. Returns `Ok(true)` to quit.
    pub fn handle_key(&mut self, port: &mut VirtualPort, key: KeyEvent) -> Result<bool> {
        if key.kind == KeyEventKind::Repeat {
            return Ok(false);
        }
        // Quit: Esc or Ctrl-C (checked before note mapping so Ctrl-C never plays).
        if key.code == KeyCode::Esc
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
        {
            return Ok(true);
        }

        if let Some(offset) = note_offset(key.code) {
            if key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
            {
                return Ok(false);
            }
            match key.kind {
                KeyEventKind::Press => self.press_note(port, offset)?,
                KeyEventKind::Release if self.note_mode == NoteMode::PressRelease => {
                    self.release_note(port, offset)?;
                }
                _ => {}
            }
            return Ok(false);
        }

        // Controls act on press only.
        if key.kind != KeyEventKind::Press {
            return Ok(false);
        }
        if let Some(action) = control_action(key.code) {
            self.apply(port, action)?;
        }
        Ok(false)
    }

    fn apply(&mut self, port: &mut VirtualPort, action: Action) -> Result<()> {
        match action {
            Action::OctaveUp => {
                self.octave = (self.octave + 1).min(OCTAVE_MAX);
                self.log = format!("octave {}", self.octave);
            }
            Action::OctaveDown => {
                self.octave = (self.octave - 1).max(OCTAVE_MIN);
                self.log = format!("octave {}", self.octave);
            }
            Action::SustainToggle => {
                self.sustain = !self.sustain;
                let value = if self.sustain { 127 } else { 0 };
                port.send_tracked(&midi::control_change(self.channel, midi::CC_SUSTAIN, value))?;
                self.log = format!("sustain {}", if self.sustain { "on" } else { "off" });
            }
            Action::AftertouchRampUp => {
                self.aftertouch_target = 1.0;
                self.aftertouch_ramping = true;
                self.log = "aftertouch ramp up".to_string();
            }
            Action::AftertouchRampDown => {
                self.aftertouch_target = 0.0;
                self.aftertouch_ramping = true;
                self.log = "aftertouch ramp down".to_string();
            }
            Action::AftertouchHold => {
                self.aftertouch_ramping = false;
                self.log = "aftertouch hold".to_string();
            }
            Action::CcNext => {
                if !self.controls.is_empty() {
                    self.cc_index = (self.cc_index + 1) % self.controls.len();
                }
            }
            Action::CcPrev => {
                if !self.controls.is_empty() {
                    self.cc_index = (self.cc_index + self.controls.len() - 1) % self.controls.len();
                }
            }
            Action::CcInc => self.adjust_cc(port, CC_STEP)?,
            Action::CcDec => self.adjust_cc(port, -CC_STEP)?,
            Action::CcSetMax => self.set_cc(port, 1.0)?,
            Action::CcSetMin => self.set_cc(port, 0.0)?,
            Action::PitchUp => {
                self.bend = (self.bend + BEND_STEP).min(1.0);
                port.send_tracked(&midi::pitch_bend_norm(self.channel, self.bend))?;
                self.log = format!("pitch bend {:+.2}", self.bend);
            }
            Action::PitchDown => {
                self.bend = (self.bend - BEND_STEP).max(-1.0);
                port.send_tracked(&midi::pitch_bend_norm(self.channel, self.bend))?;
                self.log = format!("pitch bend {:+.2}", self.bend);
            }
            Action::PitchCenter => {
                self.bend = 0.0;
                port.send_tracked(&midi::pitch_bend_norm(self.channel, self.bend))?;
                self.log = "pitch bend centre".to_string();
            }
            Action::Program(program) => {
                port.send_tracked(&midi::program_change(self.channel, program))?;
                self.program = Some(program);
                self.log = format!("program change {program}");
            }
            Action::Panic => self.panic(port)?,
            Action::Fire => self.fire(port),
            Action::NoteModeCycle => self.cycle_note_mode(),
        }
        self.dirty = true;
        Ok(())
    }

    fn adjust_cc(&mut self, port: &mut VirtualPort, delta: f32) -> Result<()> {
        let current = self.cc_values.get(self.cc_index).copied().unwrap_or(0.0);
        let value = (current + delta).clamp(0.0, 1.0);
        self.set_cc(port, value)
    }

    fn set_cc(&mut self, port: &mut VirtualPort, value: f32) -> Result<()> {
        let Some((name, cc)) = self.controls.get(self.cc_index).cloned() else {
            return Ok(());
        };
        let value = value.clamp(0.0, 1.0);
        self.cc_values[self.cc_index] = value;
        port.send_tracked(&midi::control_change(self.channel, cc, midi::clamp7(value)))?;
        self.log = format!("{name} = {:.0}%", value * 100.0);
        Ok(())
    }

    fn press_note(&mut self, port: &mut VirtualPort, offset: u8) -> Result<()> {
        let note = self.note_of(offset);
        match self.note_mode {
            NoteMode::PressRelease => {
                port.send_tracked(&midi::note_on(self.channel, note, VELOCITY))?;
                self.held.insert(note);
            }
            NoteMode::Gate => {
                port.send_tracked(&midi::note_on(self.channel, note, VELOCITY))?;
                self.held.insert(note);
                // Time the gate from the actual keypress, not the (up to one
                // tick stale) `last_tick`, so short gates aren't cut short.
                self.scheduled.push(Scheduled {
                    at: Instant::now() + self.gate,
                    bytes: midi::note_off(self.channel, note).to_vec(),
                    clears_note: Some(note),
                });
            }
            NoteMode::Latch => {
                if self.held.remove(&note) {
                    port.send_tracked(&midi::note_off(self.channel, note))?;
                } else {
                    port.send_tracked(&midi::note_on(self.channel, note, VELOCITY))?;
                    self.held.insert(note);
                }
            }
        }
        self.dirty = true;
        Ok(())
    }

    fn release_note(&mut self, port: &mut VirtualPort, offset: u8) -> Result<()> {
        let note = self.note_of(offset);
        if self.held.remove(&note) {
            port.send_tracked(&midi::note_off(self.channel, note))?;
            self.dirty = true;
        }
        Ok(())
    }

    fn panic(&mut self, port: &mut VirtualPort) -> Result<()> {
        port.panic()?;
        self.held.clear();
        self.scheduled.clear();
        self.sustain = false;
        self.aftertouch = 0.0;
        self.aftertouch_target = 0.0;
        self.aftertouch_ramping = false;
        self.last_aftertouch_sent = 0;
        self.bend = 0.0;
        self.log = "PANIC — all notes off, controllers reset".to_string();
        Ok(())
    }

    fn fire(&mut self, _port: &mut VirtualPort) {
        let now = Instant::now();
        let (name, count) = match self.fired.as_ref() {
            Some(fired) => {
                let scheduled: Vec<Scheduled> = fired
                    .events
                    .iter()
                    .map(|event| Scheduled {
                        at: now + Duration::from_millis(event.at_ms),
                        bytes: event.bytes.clone(),
                        clears_note: None,
                    })
                    .collect();
                let count = scheduled.len();
                self.scheduled.extend(scheduled);
                (fired.name.clone(), count)
            }
            None => {
                self.log = "no scenario preloaded (pass a path to `live`)".to_string();
                return;
            }
        };
        self.log = format!("firing {name} ({count} events)");
    }

    fn cycle_note_mode(&mut self) {
        self.note_mode = match self.note_mode {
            NoteMode::PressRelease => NoteMode::Gate,
            NoteMode::Gate => NoteMode::Latch,
            NoteMode::Latch if self.release_available => NoteMode::PressRelease,
            NoteMode::Latch => NoteMode::Gate,
        };
        self.log = format!("note mode: {}", self.note_mode.label());
    }

    fn note_of(&self, offset: u8) -> u8 {
        (self.octave * 12 + i32::from(offset)).clamp(0, 127) as u8
    }
}

/// Map a note key to its semitone offset within the octave (two-row layout).
fn note_offset(code: KeyCode) -> Option<u8> {
    let KeyCode::Char(character) = code else {
        return None;
    };
    match character.to_ascii_lowercase() {
        'z' => Some(0),
        's' => Some(1),
        'x' => Some(2),
        'd' => Some(3),
        'c' => Some(4),
        'v' => Some(5),
        'g' => Some(6),
        'b' => Some(7),
        'h' => Some(8),
        'n' => Some(9),
        'j' => Some(10),
        'm' => Some(11),
        ',' => Some(12),
        _ => None,
    }
}

fn control_action(code: KeyCode) -> Option<Action> {
    match code {
        KeyCode::Char('-') | KeyCode::Char('_') => Some(Action::OctaveDown),
        KeyCode::Char('=') | KeyCode::Char('+') => Some(Action::OctaveUp),
        KeyCode::Char(' ') => Some(Action::SustainToggle),
        KeyCode::Char('w') => Some(Action::AftertouchRampUp),
        KeyCode::Char('q') => Some(Action::AftertouchRampDown),
        KeyCode::Char('e') => Some(Action::AftertouchHold),
        KeyCode::Char('f') => Some(Action::Fire),
        KeyCode::Char('0') => Some(Action::PitchCenter),
        KeyCode::Char(digit @ '1'..='8') => Some(Action::Program(digit as u8 - b'1')),
        KeyCode::Backspace => Some(Action::Panic),
        KeyCode::Tab => Some(Action::CcNext),
        KeyCode::BackTab => Some(Action::CcPrev),
        KeyCode::Left => Some(Action::CcDec),
        KeyCode::Right => Some(Action::CcInc),
        KeyCode::Up => Some(Action::PitchUp),
        KeyCode::Down => Some(Action::PitchDown),
        KeyCode::PageUp => Some(Action::CcSetMax),
        KeyCode::PageDown => Some(Action::CcSetMin),
        KeyCode::Insert => Some(Action::NoteModeCycle),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::*;
    use std::time::Instant;

    fn test_state(release_available: bool) -> LiveState {
        let profile = Profile::from_toml(
            "name=\"t\"\nversion=1\n[[binding]]\ncontrol=\"K8 GFM Gate\"\ncc=3\n",
            "t",
        )
        .expect("profile parses");
        LiveState::new(
            MidiChannel::new(2).expect("channel"),
            "mamut-seq",
            &profile,
            150,
            release_available,
            None,
            Instant::now(),
        )
    }

    #[test]
    fn note_offsets_cover_the_two_row_octave() {
        assert_eq!(note_offset(KeyCode::Char('z')), Some(0));
        assert_eq!(note_offset(KeyCode::Char('s')), Some(1));
        assert_eq!(note_offset(KeyCode::Char('m')), Some(11));
        assert_eq!(note_offset(KeyCode::Char(',')), Some(12));
        assert_eq!(note_offset(KeyCode::Char('1')), None);
        assert_eq!(note_offset(KeyCode::Esc), None);
    }

    #[test]
    fn note_of_tracks_the_octave() {
        let state = test_state(false);
        assert_eq!(state.note_of(0), 60); // octave 5 -> middle C
        assert_eq!(state.note_of(12), 72);
    }

    #[test]
    fn note_mode_defaults_and_cycles_by_capability() {
        let mut kitty = test_state(true);
        assert_eq!(kitty.note_mode(), NoteMode::PressRelease);
        kitty.cycle_note_mode();
        assert_eq!(kitty.note_mode(), NoteMode::Gate);
        kitty.cycle_note_mode();
        assert_eq!(kitty.note_mode(), NoteMode::Latch);
        kitty.cycle_note_mode();
        assert_eq!(kitty.note_mode(), NoteMode::PressRelease);

        let mut plain = test_state(false);
        assert_eq!(plain.note_mode(), NoteMode::Gate);
        plain.cycle_note_mode();
        assert_eq!(plain.note_mode(), NoteMode::Latch);
        plain.cycle_note_mode();
        assert_eq!(plain.note_mode(), NoteMode::Gate); // never PressRelease without release
    }

    #[test]
    fn control_action_maps_program_and_panic() {
        assert!(matches!(
            control_action(KeyCode::Char('1')),
            Some(Action::Program(0))
        ));
        assert!(matches!(
            control_action(KeyCode::Char('8')),
            Some(Action::Program(7))
        ));
        assert!(matches!(
            control_action(KeyCode::Backspace),
            Some(Action::Panic)
        ));
        assert!(control_action(KeyCode::Char('z')).is_none()); // note key, not a control
    }
}
