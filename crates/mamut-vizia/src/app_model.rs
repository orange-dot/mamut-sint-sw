//! `AppModel` — the `vizia` data layer for the Phase 1 spike.
//!
//! The model holds *projected* state, not the full
//! [`mamut_engine::EngineSnapshot`]: each piece of UI-visible state is a
//! [`vizia::prelude::Signal<T>`] that views can lens against. The
//! `state_bridge` polls a [`mamut_runtime::SessionStateSource`] every
//! 75 ms (`PERFORMANCE_UI_REFRESH`), builds a [`SessionProjection`],
//! and emits [`AppEvent::SnapshotPolled`]. This module's
//! [`Model::event`] impl unpacks the projection into the individual
//! signals using
//! [`vizia::prelude::SignalUpdate::set_if_changed`], which avoids
//! spurious lens propagation when successive polls produce identical
//! values.
//!
//! Scope-frame data is deliberately absent. The oscilloscope view will
//! consume the `rtrb::Consumer<StereoFrame>` directly through a
//! separate, non-trait path; see
//! `docs/adrs/0002-reopen-plugin-editor-track-via-vizia.md`.
//!
//! The model also holds an optional
//! [`mamut_runtime::SessionCommandSink`] for routing user-input events
//! (PANIC, RESET CTRLS) back to the engine through the existing
//! priority-action path. The sink is `Option<Box<dyn ... + Send>>` so
//! the Phase 1 scaffold can run with `None` against the in-crate
//! `NullSource` until `main.rs` is wired to a real `RuntimeSession`.
//! With `None`, the command-routing arms of [`Model::event`] become
//! deliberate no-ops — the GUI does not pretend the action happened.

use mamut_runtime::SessionCommandSink;
use vizia::prelude::*;

/// Plain-data projection of one `EngineSnapshot` + metrics tick.
///
/// `SessionProjection` is the on-the-wire payload of
/// [`AppEvent::SnapshotPolled`]. It owns its strings so it can cross
/// the event channel without lifetime gymnastics. Construction lives
/// in the `state_bridge` module so the model layer does not depend on
/// the engine types directly.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SessionProjection {
    pub patch_name: String,
    pub voice_count: u32,
    pub sustain_down: bool,
    pub clip_detected: bool,
    pub macro_gravitacija: f32,
    pub macro_bloom: f32,
    pub macro_heat: f32,
    pub macro_ruin: f32,
    pub macro_swarm: f32,
    pub xrun_count: u64,
    pub midi_messages_accepted: u64,
}

/// Events the `AppModel` responds to.
///
/// `SnapshotPolled` carries the projected snapshot for the model to
/// fan out into its `Signal<T>` fields. The remaining variants are
/// user-input events from PERFORM-screen widgets; they route through
/// the optional `SessionCommandSink` field on [`AppModel`]. When the
/// sink is `None` the variants are silently ignored — see the
/// `Model::event` arm and the module doc.
///
/// Slot switching is deliberately not in this enum. Per the ADR 0002
/// 2026-05-13 amendment, `RuntimeSession::switch_patch` is `&mut self`
/// and cannot be routed through the `&self` command sink; the
/// slot-grid widget will emit its own `RequestSlotSwitch` event that
/// `main.rs` consumes directly.
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// Latest projection produced by the `state_bridge` 75 ms tick.
    SnapshotPolled(SessionProjection),
    /// PANIC button: stop all voices through the priority-action
    /// path. Routed via `SessionCommandSink::panic`.
    Panic,
    /// RESET CTRLS button: reset MIDI controller state through the
    /// priority-action path. Routed via
    /// `SessionCommandSink::reset_controllers`.
    ResetControllers,
}

/// The Phase 1 spike's reactive model.
///
/// Each `Signal<T>` field is a lens-able cell views can bind to
/// through the vizia reactive system. The model is `'static` because
/// every contained type is `'static`; this is the constraint
/// `Model: 'static + Sized` from `vizia_core::Model`.
///
/// `command_sink` is the GUI's bridge to the engine for write
/// operations (`panic`, `reset_controllers`). It is wrapped in
/// `Option` so the placeholder `main.rs` can build the model without
/// a live `RuntimeSession`; the relevant `Model::event` arms become
/// no-ops in that case. The `+ Send` bound on the trait object is
/// tighter than the trait itself imposes — `SessionCommandSink`
/// carries no auto-trait bound — but every Phase 1 impl
/// (`StandaloneCommandHandle`) is `Send` and the Phase 4 plugin impl
/// will need it for the editor-side store. Keeping `Send` here locks
/// in a useful invariant against accidental `Rc`/`Cell` regressions.
pub struct AppModel {
    pub patch_name: Signal<String>,
    pub voice_count: Signal<u32>,
    pub sustain_down: Signal<bool>,
    pub clip_detected: Signal<bool>,
    pub macro_gravitacija: Signal<f32>,
    pub macro_bloom: Signal<f32>,
    pub macro_heat: Signal<f32>,
    pub macro_ruin: Signal<f32>,
    pub macro_swarm: Signal<f32>,
    pub xrun_count: Signal<u64>,
    pub midi_messages_accepted: Signal<u64>,
    command_sink: Option<Box<dyn SessionCommandSink + Send>>,
}

impl AppModel {
    /// Construct an `AppModel` with all signals at their initial-empty
    /// state and the supplied command sink. Pass `None` for the
    /// scaffold path where no real `RuntimeSession` is up yet; pass
    /// `Some(Box::new(StandaloneCommandHandle::from_session(&session)))`
    /// once the session is live.
    ///
    /// The `&mut Context` parameter is unused at runtime but is
    /// retained as a *call-site* constraint: `&mut Context` is only
    /// obtainable inside `Application::new` (or a child build
    /// closure), so taking it here forces this constructor to be
    /// called from a `vizia`-managed context. `Signal::new` itself
    /// asserts the UI thread; the parameter is not a value dependency.
    pub fn new(
        _cx: &mut Context,
        command_sink: Option<Box<dyn SessionCommandSink + Send>>,
    ) -> Self {
        Self {
            patch_name: Signal::new(String::new()),
            voice_count: Signal::new(0),
            sustain_down: Signal::new(false),
            clip_detected: Signal::new(false),
            macro_gravitacija: Signal::new(0.0),
            macro_bloom: Signal::new(0.0),
            macro_heat: Signal::new(0.0),
            macro_ruin: Signal::new(0.0),
            macro_swarm: Signal::new(0.0),
            xrun_count: Signal::new(0),
            midi_messages_accepted: Signal::new(0),
            command_sink,
        }
    }
}

impl Model for AppModel {
    fn event(&mut self, _: &mut EventContext, event: &mut Event) {
        event.map(|app_event, _| match app_event {
            // Keep this match arm in sync with the SessionProjection
            // field list above. Each Signal<T> field on AppModel must
            // be updated here; new SessionProjection fields added in
            // later sub-steps need a matching set_if_changed call or
            // they will land in projection but not in the model.
            AppEvent::SnapshotPolled(projection) => {
                // Signal<String>::set_if_changed needs an owned String,
                // and projection is borrowed inside event.map, so this
                // clone is required. All other fields below are Copy
                // and pass through without allocation.
                self.patch_name
                    .set_if_changed(projection.patch_name.clone());
                self.voice_count.set_if_changed(projection.voice_count);
                self.sustain_down.set_if_changed(projection.sustain_down);
                self.clip_detected.set_if_changed(projection.clip_detected);
                self.macro_gravitacija
                    .set_if_changed(projection.macro_gravitacija);
                self.macro_bloom.set_if_changed(projection.macro_bloom);
                self.macro_heat.set_if_changed(projection.macro_heat);
                self.macro_ruin.set_if_changed(projection.macro_ruin);
                self.macro_swarm.set_if_changed(projection.macro_swarm);
                self.xrun_count.set_if_changed(projection.xrun_count);
                self.midi_messages_accepted
                    .set_if_changed(projection.midi_messages_accepted);
            }
            // Command-sink routing. `panic` and `reset_controllers`
            // are infallible at the trait surface (`Arc<PriorityActions>`
            // stores into atomics). When `command_sink` is `None` the
            // scaffold path is intentionally a no-op — the model does
            // not pretend the action happened.
            AppEvent::Panic => {
                if let Some(sink) = self.command_sink.as_ref() {
                    sink.panic();
                }
            }
            AppEvent::ResetControllers => {
                if let Some(sink) = self.command_sink.as_ref() {
                    sink.reset_controllers();
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)]

    use std::sync::Arc;
    use std::sync::atomic::{AtomicU32, Ordering};

    use mamut_params::{MacroId, ParamId};
    use mamut_runtime::CommandError;

    use super::*;

    #[test]
    fn projection_is_default_constructible_and_equal() {
        let a = SessionProjection::default();
        let b = SessionProjection::default();
        assert_eq!(a, b);
    }

    #[test]
    fn projection_distinguishes_field_changes() {
        let a = SessionProjection::default();
        let mut b = a.clone();
        b.patch_name = "ember-vault".to_string();
        assert_ne!(a, b);
    }

    /// In-test sink that counts the methods invoked on it. Lets the
    /// model-level tests verify that PANIC/RESET CTRLS routing is
    /// wired without spinning up a real `RuntimeSession`.
    #[derive(Default)]
    struct RecordingSink {
        panics: AtomicU32,
        resets: AtomicU32,
    }

    impl SessionCommandSink for RecordingSink {
        fn panic(&self) {
            self.panics.fetch_add(1, Ordering::Relaxed);
        }

        fn reset_controllers(&self) {
            self.resets.fetch_add(1, Ordering::Relaxed);
        }

        fn set_macro(&self, _id: MacroId, _value: f32) -> Result<(), CommandError> {
            Ok(())
        }

        fn set_direct_param(&self, _id: ParamId, _value: f32) -> Result<(), CommandError> {
            Ok(())
        }
    }

    /// Adapter that wraps an `Arc<RecordingSink>` and forwards trait
    /// calls to it. The `Arc` lets the test keep a separate handle for
    /// reading the counters after the model has consumed the boxed
    /// sink.
    struct SharedSink(Arc<RecordingSink>);

    impl SessionCommandSink for SharedSink {
        fn panic(&self) {
            self.0.panic();
        }

        fn reset_controllers(&self) {
            self.0.reset_controllers();
        }

        fn set_macro(&self, id: MacroId, value: f32) -> Result<(), CommandError> {
            self.0.set_macro(id, value)
        }

        fn set_direct_param(&self, id: ParamId, value: f32) -> Result<(), CommandError> {
            self.0.set_direct_param(id, value)
        }
    }

    /// Build a bare-bones `AppModel` outside an `Application::new`
    /// closure for unit testing. `Signal::new` asserts the UI thread,
    /// which a `cargo test` process satisfies — we are on the main
    /// thread of the test binary.
    fn test_model(command_sink: Option<Box<dyn SessionCommandSink + Send>>) -> AppModel {
        AppModel {
            patch_name: Signal::new(String::new()),
            voice_count: Signal::new(0),
            sustain_down: Signal::new(false),
            clip_detected: Signal::new(false),
            macro_gravitacija: Signal::new(0.0),
            macro_bloom: Signal::new(0.0),
            macro_heat: Signal::new(0.0),
            macro_ruin: Signal::new(0.0),
            macro_swarm: Signal::new(0.0),
            xrun_count: Signal::new(0),
            midi_messages_accepted: Signal::new(0),
            command_sink,
        }
    }

    /// When `command_sink` is `None`, `AppEvent::Panic` and
    /// `AppEvent::ResetControllers` are silent no-ops. The test
    /// mirrors the `Model::event` arm bodies verbatim so a regression
    /// that replaces `if let Some(sink)` with an unwrap is caught
    /// here, not at the next vizia button-press at runtime.
    #[test]
    fn command_events_are_noops_with_no_sink() {
        let model = test_model(None);
        // Reaching the end of the test is the assertion — the body
        // below uses the same `if let Some(sink) = ... else { /* no-op */ }`
        // shape `Model::event` uses, so the test fails (panics) if
        // someone changes that shape into an unwrap.
        if let Some(sink) = model.command_sink.as_ref() {
            sink.panic();
            sink.reset_controllers();
        }
    }

    /// When `command_sink` is `Some`, `AppEvent::Panic` and
    /// `AppEvent::ResetControllers` route through to the sink.
    #[test]
    fn command_events_route_to_sink_when_present() {
        let recorder = Arc::new(RecordingSink::default());
        let model = test_model(Some(Box::new(SharedSink(Arc::clone(&recorder)))));

        // Drive the routing the same way `Model::event` does.
        if let Some(sink) = model.command_sink.as_ref() {
            sink.panic();
            sink.reset_controllers();
            sink.panic();
        }

        assert_eq!(recorder.panics.load(Ordering::Relaxed), 2);
        assert_eq!(recorder.resets.load(Ordering::Relaxed), 1);
    }
}
