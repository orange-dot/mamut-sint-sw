//! `MacroMeter` — horizontal-bar view for one identity macro.
//!
//! Phase 1 deliberately uses a horizontal bar instead of the rotary
//! knob in the wireframe (`docs/ui/vizia-design-system.md`): the
//! rotary-knob custom view is Phase 3 widget work and is not in the
//! Phase 1 minimum widget set per ADR 0002.
//!
//! Each instance binds to one of the five `Signal<f32>` macro fields
//! on [`crate::app_model::AppModel`] (`macro_gravitacija`,
//! `macro_bloom`, `macro_heat`, `macro_ruin`, `macro_swarm`). The
//! bound signal is the *effective* macro value already produced by the
//! engine (post-curve, post-clamping) — the bar paints `[0.0, 1.0]`
//! linearly into the track width and prints the same value to two
//! decimal places.
//!
//! Style intent (instrument-panel matte; see design spec §3 palette):
//! the track is a dark blue-gray, the fill is cyan accent. No glow, no
//! gradient — Phase 1 is layout and binding plumbing, not aesthetic
//! polish.
//!
//! Allocation: `Signal::map(|v| format!("{:.2}", v))` allocates one
//! `String` per change. This is the GUI tick (75 ms cadence), not the
//! audio path. Allocation pressure here is acceptable; the workspace
//! lint posture forbids it on render-time code but not on the GUI.

use vizia::prelude::*;

use super::style::{FILL_ACCENT, TRACK_BG};

/// One row in the macros panel: `label` (left) + horizontal bar
/// (middle) + numeric value (right). Bound to a `Signal<f32>` whose
/// value is expected to be in `[0.0, 1.0]`.
pub struct MacroMeter;

impl MacroMeter {
    /// Build a `MacroMeter` row. The `label` is rendered once at
    /// construction (the macro names are static spec, not reactive);
    /// the bar fill and the numeric value text are reactive on `value`.
    pub fn new(cx: &mut Context, label: impl Into<String>, value: Signal<f32>) -> Handle<'_, Self> {
        let label = label.into();
        Self.build(cx, move |cx| {
            HStack::new(cx, |cx| {
                Label::new(cx, label).width(Pixels(96.0));

                // Track + fill. The fill is a child Element whose
                // width is bound to the macro value as a percentage of
                // the track; the track itself has a fixed pixel width.
                //
                // Asymmetric handling of out-of-range values: the
                // fill-width clamps to `[0.0, 1.0]` so a contract
                // violation cannot break layout. The numeric label
                // below does *not* clamp, so an upstream bug producing
                // a value outside the range surfaces visually in the
                // readout rather than being silently hidden.
                HStack::new(cx, |cx| {
                    Element::new(cx)
                        .width(value.map(|v| Percentage(v.clamp(0.0, 1.0) * 100.0)))
                        .height(Stretch(1.0))
                        .background_color(FILL_ACCENT);
                })
                .width(Pixels(200.0))
                .height(Pixels(8.0))
                .background_color(TRACK_BG);

                Label::new(cx, value.map(|v| format!("{:.2}", v))).width(Pixels(48.0));
            })
            .height(Pixels(24.0))
            .horizontal_gap(Pixels(8.0));
        })
    }
}

impl View for MacroMeter {}
