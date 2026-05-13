//! `StateBridge` — drives the 75 ms poll from a
//! [`mamut_runtime::SessionStateSource`] into [`AppEvent`].
//!
//! On each tick the bridge calls
//! [`SessionStateSource::poll_snapshot`], snapshots the published
//! metrics counters, builds a [`SessionProjection`], and emits
//! [`AppEvent::SnapshotPolled`]. The 75 ms cadence matches
//! `mamut_runtime::PERFORMANCE_UI_REFRESH` and is fixed by ADR 0002.
//!
//! The bridge owns its source as `Box<dyn SessionStateSource>`, which
//! requires the contained source to be `'static`. The borrowed
//! [`mamut_runtime::StandaloneSessionSource<'a>`] does *not* satisfy
//! that bound; production wiring uses
//! [`mamut_runtime::StandaloneSessionHandle`] instead, which clones the
//! command channel `Sender` and the three `Arc<*Metrics>` from the
//! owning `RuntimeSession` and is `'static + Send + Sync`. Construct it
//! via `StandaloneSessionHandle::from_session(&runtime_session)` once
//! the runtime is up. The `NullSource` scaffold below is kept for tests
//! and the blank-window placeholder in `main.rs`; the eventual `play`
//! entry point replaces it with the real handle.
//!
//! Scope frames are not handled here; the oscilloscope widget will
//! own its `rtrb::Consumer<StereoFrame>` directly.

use std::sync::Arc;
use std::time::Duration;

use mamut_engine::EngineSnapshot;
use mamut_runtime::{
    InputMetrics, PERFORMANCE_UI_REFRESH, RecordingMetrics, SessionStateSource, TransportMetrics,
};
use vizia::prelude::*;

use crate::app_model::{AppEvent, SessionProjection};

/// Polling interval; mirrors `PERFORMANCE_UI_REFRESH` exactly. Defined
/// here as a `const` so it is visible in this file's docs and tests
/// without crossing the runtime crate boundary at every callsite.
pub const POLL_INTERVAL: Duration = PERFORMANCE_UI_REFRESH;

/// Owns a [`SessionStateSource`] and drives the 75 ms snapshot poll on
/// the `vizia` timer.
pub struct StateBridge {
    source: Box<dyn SessionStateSource>,
}

impl StateBridge {
    /// Build a bridge over the given source.
    pub fn new(source: Box<dyn SessionStateSource>) -> Self {
        Self { source }
    }

    /// Build a [`SessionProjection`] from one tick of the underlying
    /// source. Returns `None` when the source itself returned `None`
    /// from `poll_snapshot` (engine outage, transient timeout); the
    /// caller is expected to render the last-known-good state and
    /// retry on the next tick.
    pub fn poll_once(&self) -> Option<SessionProjection> {
        project(self.source.as_ref())
    }

    /// Install a `vizia` timer that calls [`Self::poll_once`] every
    /// 75 ms and emits [`AppEvent::SnapshotPolled`] for each successful
    /// poll. The bridge is moved into the timer's closure; it lives as
    /// long as the timer is registered. Returns the [`Timer`] handle so
    /// the caller may halt dispatch on shutdown.
    ///
    /// **Timer lifetime.** `add_timer` is called with `None` as the
    /// total duration, which means the timer runs indefinitely. Callers
    /// must invoke `cx.stop_timer(timer)` on shutdown to stop dispatch.
    /// Stopping the timer halts ticks but does **not** by itself drop
    /// the captured `self`; the bridge (and its
    /// [`Box<dyn SessionStateSource>`]) lives until `vizia` reclaims
    /// the timer slot (typically on `Context` teardown). For Phase 1
    /// with [`NullSource`] this is benign — the underlying boxed
    /// source's [`Drop`] is trivial. When Step 3 sub-step 1.5 lands the
    /// owned-handle source variant, callers may need a dedicated
    /// teardown helper if the source's `Drop` participates in graceful
    /// shutdown.
    ///
    /// **Silent `None` ticks.** If `poll_snapshot` returns `None`
    /// (engine outage, blocking-snapshot timeout, runtime shutdown),
    /// the tick is dropped without emitting an event. Per ADR 0002 the
    /// GUI is expected to render the last-known-good projection until
    /// the next successful tick; outage observability comes from the
    /// metrics counters, which the bridge re-reads on every tick and
    /// which remain reachable even when `poll_snapshot` fails.
    pub fn install(self, cx: &mut Context) -> Timer {
        let timer = cx.add_timer(POLL_INTERVAL, None, move |cx, action| {
            if let TimerAction::Tick(_) = action {
                if let Some(projection) = self.poll_once() {
                    cx.emit(AppEvent::SnapshotPolled(projection));
                }
            }
        });
        cx.start_timer(timer);
        timer
    }
}

/// Build a [`SessionProjection`] from a [`SessionStateSource`].
///
/// Free function so it is unit-testable without owning a real
/// [`StateBridge`]. Returns `None` if the source produces no snapshot.
pub(crate) fn project(source: &dyn SessionStateSource) -> Option<SessionProjection> {
    let snapshot = source.poll_snapshot()?;
    let transport = source.transport_metrics().snapshot();
    let input = source.input_metrics().snapshot();
    Some(SessionProjection {
        // SessionProjection owns its strings to decouple from the
        // engine-side EngineSnapshot lifetime; this clone is the GUI
        // tick path, not the audio fast path.
        patch_name: snapshot.patch_name.clone(),
        // active_voice_count is a usize counter on the voice allocator;
        // saturation past u32::MAX is physically impossible for any
        // real voice budget. unwrap_or is defensive against the cast
        // signature, not load-bearing.
        voice_count: u32::try_from(snapshot.active_voice_count).unwrap_or(u32::MAX),
        sustain_down: snapshot.sustain_down,
        clip_detected: snapshot.clip_detected,
        macro_gravitacija: snapshot.effective_macros.gravitacija,
        macro_bloom: snapshot.effective_macros.bloom,
        macro_heat: snapshot.effective_macros.heat,
        macro_ruin: snapshot.effective_macros.ruin,
        macro_swarm: snapshot.effective_macros.swarm,
        xrun_count: transport.xrun_recoveries,
        midi_messages_accepted: input.midi_messages_accepted,
    })
}

/// A `SessionStateSource` that never produces a snapshot.
///
/// Used by the Phase 1 spike scaffold in `main.rs` (the blank-window
/// placeholder; no real `RuntimeSession` is up yet) and by this
/// module's tests as a deterministic empty source. The production path
/// is [`mamut_runtime::StandaloneSessionHandle`] built from a live
/// session via `from_session(&runtime_session)`. The metrics handles
/// here are freshly-constructed `Arc<*Metrics>` so a future test can
/// mutate the counters to verify projection plumbing.
pub struct NullSource {
    transport: Arc<TransportMetrics>,
    input: Arc<InputMetrics>,
    recording: Arc<RecordingMetrics>,
}

impl NullSource {
    pub fn new() -> Self {
        Self {
            transport: Arc::new(TransportMetrics::default()),
            input: Arc::new(InputMetrics::default()),
            recording: Arc::new(RecordingMetrics::default()),
        }
    }
}

impl Default for NullSource {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionStateSource for NullSource {
    fn poll_snapshot(&self) -> Option<EngineSnapshot> {
        None
    }

    fn transport_metrics(&self) -> &Arc<TransportMetrics> {
        &self.transport
    }

    fn input_metrics(&self) -> &Arc<InputMetrics> {
        &self.input
    }

    fn recording_metrics(&self) -> &Arc<RecordingMetrics> {
        &self.recording
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test coverage gap: the project() Some-path — i.e. mapping every
    // SessionProjection field from a real EngineSnapshot — has no test
    // here. A fixture would require constructing EngineSnapshot
    // manually, which is impractical at this layer because
    // mamut_engine::DirectParameters has ~96 fields with no Default and
    // several other snapshot sub-types lack Default too. The
    // StandaloneSessionHandle from sub-step 1.5 does not change this —
    // it forwards through the same EngineSnapshot. The natural place
    // for this test is an integration test that drives a real engine
    // through one tick once Step 3 sub-step 2+ widgets need it. The
    // alternative (adding #[derive(Default)] across mamut-engine
    // snapshot sub-types) is a separate, in-scope-for-its-own-commit
    // change.

    #[test]
    fn project_returns_none_when_source_has_no_snapshot() {
        let source = NullSource::new();
        assert!(project(&source).is_none());
    }

    #[test]
    fn poll_once_forwards_none_from_null_source() {
        let bridge = StateBridge::new(Box::new(NullSource::new()));
        assert!(bridge.poll_once().is_none());
    }

    #[test]
    fn poll_interval_matches_performance_ui_refresh() {
        assert_eq!(POLL_INTERVAL, PERFORMANCE_UI_REFRESH);
    }
}
