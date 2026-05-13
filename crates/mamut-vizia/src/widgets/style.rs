//! Shared Phase 1 palette for PERFORM-screen widgets.
//!
//! Single-source-of-truth for the matte instrument-panel colors used
//! by `macro_meter`, `perform_rail`, and the upcoming `slot_grid` /
//! `rotary_knob` / `oscilloscope` widgets. Lifting these out of each
//! widget file keeps the palette stable as Phase 3 fills in the final
//! design tokens; the design spec
//! (`docs/ui/vizia-design-system.md`) is the upstream truth, and this
//! file is the place to fold its values in once they're frozen.

use vizia::prelude::*;

// Track + fill for horizontal bars (macro meters).
pub const TRACK_BG: Color = Color::rgb(42, 46, 53);
pub const FILL_ACCENT: Color = Color::rgb(92, 242, 230);

// Persistent rail chrome.
pub const RAIL_BG: Color = Color::rgb(34, 38, 45);
pub const RAIL_TEXT: Color = Color::rgb(220, 224, 232);

// Badges and action buttons.
pub const BADGE_DIM: Color = Color::rgb(74, 80, 92);
pub const BADGE_ACTIVE: Color = Color::rgb(255, 195, 80);
pub const BADGE_WARN: Color = Color::rgb(232, 92, 92);
pub const BUTTON_BG: Color = Color::rgb(58, 64, 76);
