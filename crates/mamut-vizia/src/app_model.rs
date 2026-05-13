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
/// For Phase 1 sub-step 1 the only event is the polled projection. User
/// input events (slot select, macro change, PANIC) will land here in
/// subsequent sub-steps; widget files own their own event variants and
/// will extend this enum.
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    SnapshotPolled(SessionProjection),
}

/// The Phase 1 spike's reactive model.
///
/// Each field is a `Signal<T>` so views can `bind` to it through the
/// vizia reactive system. The model is `'static` because every contained
/// type is `'static`; this is the constraint `Model: 'static + Sized`
/// from `vizia_core::Model`.
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
}

impl AppModel {
    /// Construct an `AppModel` with all signals at their initial-empty
    /// state. The `&mut Context` parameter is unused at runtime but is
    /// retained as a *call-site* constraint: `&mut Context` is only
    /// obtainable inside `Application::new` (or a child build closure),
    /// so taking it here forces this constructor to be called from a
    /// `vizia`-managed context. `Signal::new` itself asserts the UI
    /// thread; the parameter is not a value dependency.
    pub fn new(_cx: &mut Context) -> Self {
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
        });
    }
}

#[cfg(test)]
mod tests {
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
}
