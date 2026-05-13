//! `PerformRail` — persistent header for the PERFORM screen.
//!
//! Phase 1 minimum surface per ADR 0002 Phase 1 minimum widget set:
//! patch name, voice count, sustain badge, clip badge, xrun count,
//! plus PANIC and RESET CTRLS action buttons. The full target list
//! from the plan ("audio device + sample rate, MIDI device + channel,
//! last control touched, live-slot index") is deferred — those
//! signals are not yet projected through `SessionProjection`, so the
//! Phase 1 rail shows only what `AppModel` already exposes.
//!
//! The buttons emit `AppEvent::Panic` and
//! `AppEvent::ResetControllers`; `AppModel::event` routes them
//! through the optional `SessionCommandSink`. When the sink is `None`
//! (the scaffold path before `play` wires a real `RuntimeSession`),
//! the emits become silent no-ops — the widget gives no false
//! confirmation that the engine acted.

use vizia::prelude::*;

use super::style::{BADGE_ACTIVE, BADGE_DIM, BADGE_WARN, BUTTON_BG, RAIL_BG, RAIL_TEXT};
use crate::app_model::AppEvent;

/// Read-state signals consumed by the rail.
///
/// The rail does not own these — they are `Signal<T>` handles from
/// [`crate::app_model::AppModel`]. Grouping them into a struct keeps
/// the constructor signature manageable. Later sub-steps that add
/// fields (audio device, MIDI device, last-control-touched) will
/// extend this struct; struct-literal callers (currently only
/// `main.rs`) will need to add the new fields, but the
/// `PerformRail::new` signature itself stays stable.
#[derive(Clone, Copy)]
pub struct PerformRailSignals {
    pub patch_name: Signal<String>,
    pub voice_count: Signal<u32>,
    pub sustain_down: Signal<bool>,
    pub clip_detected: Signal<bool>,
    pub xrun_count: Signal<u64>,
}

pub struct PerformRail;

impl PerformRail {
    /// Build the persistent rail. The bound signals drive the
    /// read-state badges and labels; PANIC and RESET CTRLS buttons
    /// emit [`AppEvent::Panic`] and [`AppEvent::ResetControllers`].
    pub fn new(cx: &mut Context, signals: PerformRailSignals) -> Handle<'_, Self> {
        Self.build(cx, move |cx| {
            HStack::new(cx, |cx| {
                // Patch label. The "patch:" prefix is static; the
                // bound signal carries the name itself.
                Label::new(cx, "patch:")
                    .color(RAIL_TEXT)
                    .width(Pixels(56.0));
                Label::new(cx, signals.patch_name)
                    .color(RAIL_TEXT)
                    .width(Pixels(160.0));

                // Voice count.
                Label::new(cx, "voices:")
                    .color(RAIL_TEXT)
                    .width(Pixels(56.0));
                Label::new(cx, signals.voice_count.map(|n| n.to_string()))
                    .color(RAIL_TEXT)
                    .width(Pixels(32.0));

                // Sustain badge — dim by default, accent when held.
                Label::new(cx, "sustain")
                    .color(RAIL_TEXT)
                    .width(Pixels(60.0))
                    .background_color(
                        signals
                            .sustain_down
                            .map(|on| if *on { BADGE_ACTIVE } else { BADGE_DIM }),
                    );

                // Clip badge — dim by default, warn-red when clipping.
                Label::new(cx, "clip")
                    .color(RAIL_TEXT)
                    .width(Pixels(40.0))
                    .background_color(
                        signals
                            .clip_detected
                            .map(|on| if *on { BADGE_WARN } else { BADGE_DIM }),
                    );

                // xrun count.
                Label::new(cx, "xruns:")
                    .color(RAIL_TEXT)
                    .width(Pixels(48.0));
                Label::new(cx, signals.xrun_count.map(|n| n.to_string()))
                    .color(RAIL_TEXT)
                    .width(Pixels(40.0));

                // PANIC + RESET CTRLS.
                Button::new(cx, |cx| Label::new(cx, "PANIC").color(RAIL_TEXT))
                    .on_press(|cx| cx.emit(AppEvent::Panic))
                    .background_color(BUTTON_BG)
                    .width(Pixels(80.0));
                Button::new(cx, |cx| Label::new(cx, "RESET CTRLS").color(RAIL_TEXT))
                    .on_press(|cx| cx.emit(AppEvent::ResetControllers))
                    .background_color(BUTTON_BG)
                    .width(Pixels(120.0));
            })
            .height(Pixels(32.0))
            .horizontal_gap(Pixels(8.0))
            .background_color(RAIL_BG);
        })
    }
}

impl View for PerformRail {}
