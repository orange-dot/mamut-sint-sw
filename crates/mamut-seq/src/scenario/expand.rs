//! Pure schedule expansion: `(ScenarioFile, Profile, default channel)` → a
//! sorted list of absolute-time MIDI events. Same inputs always yield an
//! identical list; this is exactly what `validate` prints and what `play`
//! schedules.
//!
//! `expand` assumes the scenario has already passed [`validate_scenario`] (the
//! callers own that step); it performs profile resolution and event generation,
//! bounded by a step/event budget so a hostile file cannot hang or OOM.

use crate::midi::{self, MidiChannel};
use crate::profile::Profile;

use super::errors::ScenarioError;
use super::model::{ScenarioFile, Step};

/// Upper bound on a single ramp/envelope's interpolation points.
const MAX_RAMP_STEPS: u32 = 4096;
/// Upper bound on `expand_step` invocations (bounds loop iterations/nesting).
const MAX_EXPANSION_STEPS: u64 = 2_000_000;
/// Upper bound on generated events (bounds ramp explosions).
const MAX_EVENTS: u64 = 500_000;

/// One fully-resolved MIDI event at an absolute offset from playback start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbsoluteEvent {
    pub at_ms: u64,
    pub bytes: Vec<u8>,
    pub label: String,
}

/// Profile-independent structural validation. Run before `expand`.
pub fn validate_scenario(scenario: &ScenarioFile) -> Result<(), ScenarioError> {
    if scenario.schema_version != 1 {
        return Err(ScenarioError::UnsupportedSchemaVersion {
            found: scenario.schema_version,
        });
    }
    if scenario.name.trim().is_empty() {
        return Err(ScenarioError::EmptyName);
    }
    if scenario.timeline.is_empty() {
        return Err(ScenarioError::EmptyTimeline);
    }
    for step in &scenario.timeline {
        validate_step(step)?;
    }
    Ok(())
}

fn validate_step(step: &Step) -> Result<(), ScenarioError> {
    match step {
        Step::Wait { .. } => Ok(()),
        Step::Cc { value, channel, .. } => {
            check_finite("cc value", *value)?;
            check_channel(*channel)
        }
        Step::ModWheel { value, channel } => {
            check_finite("mod_wheel value", *value)?;
            check_channel(*channel)
        }
        Step::PitchBend { value, channel } => {
            check_finite("pitch_bend value", *value)?;
            check_channel(*channel)
        }
        Step::Aftertouch { pressure, channel } => {
            check_finite("aftertouch pressure", *pressure)?;
            check_channel(*channel)
        }
        Step::Sustain { channel, .. } => check_channel(*channel),
        Step::Note {
            note,
            velocity,
            channel,
            ..
        }
        | Step::NoteOn {
            note,
            velocity,
            channel,
            ..
        } => {
            check_note(*note)?;
            check_velocity(*velocity)?;
            check_channel(*channel)
        }
        Step::NoteOff { note, channel } => {
            check_note(*note)?;
            check_channel(*channel)
        }
        Step::Chord {
            notes,
            velocity,
            channel,
            ..
        } => {
            if notes.is_empty() {
                return Err(ScenarioError::EmptyChord);
            }
            for note in notes {
                check_note(*note)?;
            }
            check_velocity(*velocity)?;
            check_channel(*channel)
        }
        Step::CcRamp {
            from,
            to,
            steps,
            channel,
            ..
        } => {
            check_finite("cc_ramp from", *from)?;
            check_finite("cc_ramp to", *to)?;
            check_ramp_steps(*steps)?;
            check_channel(*channel)
        }
        Step::AftertouchEnvelope {
            peak,
            steps,
            channel,
            ..
        } => {
            check_finite("aftertouch_envelope peak", *peak)?;
            check_ramp_steps(*steps)?;
            check_channel(*channel)
        }
        Step::ProgramChange { program, channel } => {
            if *program > 7 {
                return Err(ScenarioError::ProgramOutOfRange { value: *program });
            }
            check_channel(*channel)
        }
        Step::Loop { count, steps } => {
            if *count == 0 {
                return Err(ScenarioError::ZeroLoopCount);
            }
            for inner in steps {
                validate_step(inner)?;
            }
            Ok(())
        }
        Step::Section { steps, .. } => {
            for inner in steps {
                validate_step(inner)?;
            }
            Ok(())
        }
    }
}

/// Expand a validated scenario into the absolute event list.
pub fn expand(
    scenario: &ScenarioFile,
    profile: &Profile,
    default_channel: MidiChannel,
) -> Result<Vec<AbsoluteEvent>, ScenarioError> {
    let mut events = Vec::new();
    let mut cursor = 0u64;
    let mut budget = MAX_EXPANSION_STEPS;
    walk(
        &scenario.timeline,
        default_channel,
        profile,
        &mut cursor,
        &mut events,
        &mut budget,
    )?;
    // Stable sort preserves deterministic insertion order among equal deadlines.
    events.sort_by_key(|event| event.at_ms);
    Ok(events)
}

fn walk(
    steps: &[Step],
    default_channel: MidiChannel,
    profile: &Profile,
    cursor: &mut u64,
    events: &mut Vec<AbsoluteEvent>,
    budget: &mut u64,
) -> Result<(), ScenarioError> {
    for step in steps {
        expand_step(step, default_channel, profile, cursor, events, budget)?;
    }
    Ok(())
}

fn expand_step(
    step: &Step,
    default_channel: MidiChannel,
    profile: &Profile,
    cursor: &mut u64,
    events: &mut Vec<AbsoluteEvent>,
    budget: &mut u64,
) -> Result<(), ScenarioError> {
    if *budget == 0 {
        return Err(ScenarioError::ExpansionBudgetExceeded {
            limit: MAX_EXPANSION_STEPS,
        });
    }
    *budget -= 1;
    if events.len() as u64 >= MAX_EVENTS {
        return Err(ScenarioError::ExpansionBudgetExceeded { limit: MAX_EVENTS });
    }

    match step {
        Step::Wait { ms } => {
            *cursor = cursor.saturating_add(*ms);
        }
        Step::Note {
            note,
            velocity,
            duration_ms,
            channel,
        } => {
            let ch = resolve_channel(*channel, default_channel)?;
            push_note_on(events, *cursor, ch, *note, *velocity);
            push_note_off(events, cursor.saturating_add(*duration_ms), ch, *note);
        }
        Step::NoteOn {
            note,
            velocity,
            channel,
        } => {
            let ch = resolve_channel(*channel, default_channel)?;
            push_note_on(events, *cursor, ch, *note, *velocity);
        }
        Step::NoteOff { note, channel } => {
            let ch = resolve_channel(*channel, default_channel)?;
            push_note_off(events, *cursor, ch, *note);
        }
        Step::Chord {
            notes,
            velocity,
            duration_ms,
            channel,
        } => {
            let ch = resolve_channel(*channel, default_channel)?;
            for note in notes {
                push_note_on(events, *cursor, ch, *note, *velocity);
            }
            for note in notes {
                push_note_off(events, cursor.saturating_add(*duration_ms), ch, *note);
            }
        }
        Step::Cc {
            control,
            value,
            channel,
        } => {
            let ch = resolve_channel(*channel, default_channel)?;
            let cc = resolve_control(profile, control)?;
            push_cc(events, *cursor, ch, control, cc, *value);
        }
        Step::CcRamp {
            control,
            from,
            to,
            duration_ms,
            steps,
            channel,
        } => {
            let ch = resolve_channel(*channel, default_channel)?;
            let cc = resolve_control(profile, control)?;
            for index in 0..=*steps {
                let value = from + (to - from) * ramp_fraction(index, *steps);
                let at = cursor.saturating_add(interp_ms(*duration_ms, index, *steps));
                push_cc(events, at, ch, control, cc, value);
            }
            *cursor = cursor.saturating_add(*duration_ms);
        }
        Step::Aftertouch { pressure, channel } => {
            let ch = resolve_channel(*channel, default_channel)?;
            push_aftertouch(events, *cursor, ch, *pressure);
        }
        Step::AftertouchEnvelope {
            peak,
            attack_ms,
            hold_ms,
            release_ms,
            steps,
            channel,
        } => {
            let ch = resolve_channel(*channel, default_channel)?;
            // Attack: 0 → just below peak. The peak endpoint is the hold event,
            // so the attack loop stops before it (no duplicate at the seam).
            if *attack_ms > 0 {
                for index in 0..*steps {
                    let value = *peak * ramp_fraction(index, *steps);
                    let at = cursor.saturating_add(interp_ms(*attack_ms, index, *steps));
                    push_aftertouch(events, at, ch, value);
                }
            }
            // Hold at peak (also the attack endpoint).
            let hold_start = cursor.saturating_add(*attack_ms);
            push_aftertouch(events, hold_start, ch, *peak);
            // Release: just below peak → 0 (index 0 = peak is the hold event).
            let release_start = hold_start.saturating_add(*hold_ms);
            for index in 1..=*steps {
                let value = *peak * (1.0 - ramp_fraction(index, *steps));
                let at = release_start.saturating_add(interp_ms(*release_ms, index, *steps));
                push_aftertouch(events, at, ch, value);
            }
            *cursor = cursor
                .saturating_add(*attack_ms)
                .saturating_add(*hold_ms)
                .saturating_add(*release_ms);
        }
        Step::PitchBend { value, channel } => {
            let ch = resolve_channel(*channel, default_channel)?;
            let bytes = midi::pitch_bend_norm(ch, *value);
            events.push(AbsoluteEvent {
                at_ms: *cursor,
                bytes: bytes.to_vec(),
                label: format!("pitch_bend ch{} value={value:.3}", ch.number()),
            });
        }
        Step::ModWheel { value, channel } => {
            let ch = resolve_channel(*channel, default_channel)?;
            push_cc(events, *cursor, ch, "mod wheel", midi::CC_MOD_WHEEL, *value);
        }
        Step::Sustain { down, channel } => {
            let ch = resolve_channel(*channel, default_channel)?;
            let value = if *down { 127 } else { 0 };
            events.push(AbsoluteEvent {
                at_ms: *cursor,
                bytes: midi::control_change(ch, midi::CC_SUSTAIN, value).to_vec(),
                label: format!(
                    "sustain ch{} {}",
                    ch.number(),
                    if *down { "down" } else { "up" }
                ),
            });
        }
        Step::ProgramChange { program, channel } => {
            let ch = resolve_channel(*channel, default_channel)?;
            let bytes = midi::program_change(ch, *program);
            events.push(AbsoluteEvent {
                at_ms: *cursor,
                bytes: bytes.to_vec(),
                label: format!("program_change ch{} program={program}", ch.number()),
            });
        }
        Step::Loop { count, steps } => {
            for _ in 0..*count {
                walk(steps, default_channel, profile, cursor, events, budget)?;
            }
        }
        Step::Section { steps, .. } => {
            walk(steps, default_channel, profile, cursor, events, budget)?;
        }
    }
    Ok(())
}

fn push_note_on(events: &mut Vec<AbsoluteEvent>, at: u64, ch: MidiChannel, note: u8, velocity: u8) {
    events.push(AbsoluteEvent {
        at_ms: at,
        bytes: midi::note_on(ch, note, velocity).to_vec(),
        label: format!("note_on ch{} note={note} vel={velocity}", ch.number()),
    });
}

fn push_note_off(events: &mut Vec<AbsoluteEvent>, at: u64, ch: MidiChannel, note: u8) {
    events.push(AbsoluteEvent {
        at_ms: at,
        bytes: midi::note_off(ch, note).to_vec(),
        label: format!("note_off ch{} note={note}", ch.number()),
    });
}

fn push_cc(
    events: &mut Vec<AbsoluteEvent>,
    at: u64,
    ch: MidiChannel,
    control: &str,
    cc: u8,
    value: f32,
) {
    let value7 = midi::clamp7(value);
    events.push(AbsoluteEvent {
        at_ms: at,
        bytes: midi::control_change(ch, cc, value7).to_vec(),
        label: format!("cc ch{} {control}(cc{cc})={value7}", ch.number()),
    });
}

fn push_aftertouch(events: &mut Vec<AbsoluteEvent>, at: u64, ch: MidiChannel, pressure: f32) {
    let pressure7 = midi::clamp7(pressure);
    events.push(AbsoluteEvent {
        at_ms: at,
        bytes: midi::channel_aftertouch(ch, pressure7).to_vec(),
        label: format!("aftertouch ch{} pressure={pressure7}", ch.number()),
    });
}

fn resolve_control(profile: &Profile, control: &str) -> Result<u8, ScenarioError> {
    profile
        .cc_for(control)
        .ok_or_else(|| ScenarioError::UnknownControl {
            control: control.to_string(),
            profile: profile.source().to_string(),
        })
}

fn resolve_channel(
    channel: Option<u8>,
    default_channel: MidiChannel,
) -> Result<MidiChannel, ScenarioError> {
    match channel {
        Some(number) => {
            MidiChannel::new(number).ok_or(ScenarioError::InvalidChannel { value: number })
        }
        None => Ok(default_channel),
    }
}

/// Linear fraction `index / steps` in `0.0..=1.0` (`steps` guaranteed non-zero).
fn ramp_fraction(index: u32, steps: u32) -> f32 {
    index as f32 / steps.max(1) as f32
}

/// Absolute-ms offset of interpolation point `index` within a `duration`-long
/// gesture split into `steps` intervals.
fn interp_ms(duration_ms: u64, index: u32, steps: u32) -> u64 {
    duration_ms.saturating_mul(u64::from(index)) / u64::from(steps.max(1))
}

fn check_channel(channel: Option<u8>) -> Result<(), ScenarioError> {
    match channel {
        Some(number) if !(1..=16).contains(&number) => {
            Err(ScenarioError::InvalidChannel { value: number })
        }
        _ => Ok(()),
    }
}

fn check_note(note: u8) -> Result<(), ScenarioError> {
    if note > 127 {
        Err(ScenarioError::NoteOutOfRange { value: note })
    } else {
        Ok(())
    }
}

fn check_velocity(velocity: u8) -> Result<(), ScenarioError> {
    if velocity > 127 {
        Err(ScenarioError::VelocityOutOfRange { value: velocity })
    } else {
        Ok(())
    }
}

fn check_ramp_steps(steps: u32) -> Result<(), ScenarioError> {
    if steps == 0 {
        Err(ScenarioError::ZeroRampSteps)
    } else if steps > MAX_RAMP_STEPS {
        Err(ScenarioError::RampStepsTooLarge {
            value: steps,
            limit: MAX_RAMP_STEPS,
        })
    } else {
        Ok(())
    }
}

fn check_finite(field: &'static str, value: f32) -> Result<(), ScenarioError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(ScenarioError::NonFiniteValue { field })
    }
}
