//! `mamut-vizia` — ADR 0002 Phase 1 spike entry point.
//!
//! This binary is the placeholder skeleton for the `vizia`-toolkit GUI
//! spike. It opens a blank window with a label and exits. The Phase 1
//! data layer ([`AppModel`], [`AppEvent`], [`state_bridge`]) is wired
//! up but driven by a [`state_bridge::NullSource`] until the
//! `RuntimeSession` integration lands (Step 3 sub-step 1.5).
//!
//! The Phase 1 PERFORM-screen widgets, the real session source, and
//! the CLI surface live in subsequent commits; see
//! `docs/adrs/0002-reopen-plugin-editor-track-via-vizia.md` and
//! `docs/ui/vizia-design-system.md`.

mod app_model;
mod state_bridge;

use vizia::prelude::*;

use crate::app_model::AppModel;
use crate::state_bridge::{NullSource, StateBridge};

fn main() -> Result<(), ApplicationError> {
    Application::new(|cx| {
        AppModel::new(cx).build(cx);
        let bridge = StateBridge::new(Box::new(NullSource::new()));
        bridge.install(cx);
        Label::new(cx, "mamut-vizia — Phase 1 spike (placeholder)");
    })
    .title("mamut-vizia (spike)")
    .inner_size((640, 360))
    .run()
}
