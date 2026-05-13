//! PERFORM-screen widgets for the ADR 0002 Phase 1 spike.
//!
//! Phase 1 widget cut (from
//! `docs/adrs/0002-reopen-plugin-editor-track-via-vizia.md`):
//!
//! - `macro_meter` — horizontal bar bound to one of the five
//!   identity-macro `Signal<f32>` fields on [`crate::app_model::AppModel`].
//!   Rotary knob variant is deferred to Phase 3.
//!
//! Later sub-steps add `perform_rail`, `slot_grid`, `oscilloscope`, and
//! one rotary knob (for master output trim, to validate the custom-
//! widget path). See ADR 0002 Phase 1 minimum widget set.

mod macro_meter;
mod perform_rail;
mod style;

pub use macro_meter::MacroMeter;
pub use perform_rail::{PerformRail, PerformRailSignals};
