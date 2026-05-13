use super::*;

/// Read-only consumer interface over already-published session state.
///
/// `SessionStateSource` is the surface that GUI view code (`vizia` per
/// ADR 0002 Phase 1, and the off-ramp `egui` redesign) reads from when
/// it needs an [`EngineSnapshot`] or a reference to one of the published
/// metrics counters. It carries no command surface, no scope-frame
/// access, no patch-load or device-control behavior.
///
/// Three properties of this trait are load-bearing per
/// `docs/adrs/0002-reopen-plugin-editor-track-via-vizia.md`:
///
/// - The standalone implementation ([`StandaloneSessionSource`]) is a
///   thin wrapper around [`RuntimeSession::request_snapshot`]. That
///   call is a blocking `mpsc` request/reply with a 500 ms timeout.
///   The `vizia` Phase 1 spike polls at the `PERFORMANCE_UI_REFRESH`
///   cadence (75 ms) defined in
///   `crates/mamut-runtime/src/types/constants.rs`.
/// - Scope-frame access is *not* part of this trait. The render-time
///   scope ring (`rtrb::Consumer`) requires `&mut` and is drained
///   through [`RuntimeSession::drain_scope_frames`] on the owner
///   thread.
/// - A push-style plugin implementation (engine writes into an
///   `Arc<AtomicSnapshot>` consumed by the editor) is an ADR 0002
///   Phase 4 prerequisite, not a Phase 1 deliverable.
///
/// **Blocking note.** A single [`poll_snapshot`](Self::poll_snapshot)
/// call on [`StandaloneSessionSource`] can block the calling thread for
/// up to 500 ms — about 6.7× the 75 ms poll cadence. Callers must not
/// invoke it from a frame-deadline-sensitive thread (the render thread
/// of a GUI toolkit, an audio callback, an IRQ handler) without an
/// off-thread bridge. The expected pattern for the `vizia` Phase 1
/// spike is a dedicated state-bridge thread (or a `vizia` timer) that
/// polls into a [`vizia::Model`]-shaped buffer; view code reads the
/// buffer.
///
/// **Auto-trait posture.** The trait deliberately carries no `Send` or
/// `Sync` bound. Phase 1 callers run on a single GUI bridge; whether a
/// given impl is `Send`/`Sync` falls out from its own fields, not from
/// the trait. [`StandaloneSessionSource`] inherits whatever
/// [`RuntimeSession`] permits through the `&RuntimeSession` borrow.
/// Phase 4 may introduce a separate trait alias or impl that adds the
/// bounds where they are load-bearing for the plugin host.
pub trait SessionStateSource {
    /// Request a fresh [`EngineSnapshot`] from the audio runtime.
    ///
    /// Returns `None` if the request fails. The trait collapses two
    /// distinct failure modes into the same `None`:
    ///
    /// - the audio runtime has shut down and will never reply again
    ///   (the `mpsc` send fails);
    /// - the runtime is alive but did not reply within 500 ms.
    ///
    /// The collapse is deliberate for Phase 1 — the GUI polls on a
    /// fixed cadence, treats `None` as "render last-known-good", and a
    /// transient missed poll is indistinguishable from a permanent
    /// outage from a single sample. A persistent outage will be visible
    /// in other surfaces (the audio device disappearing, the transport
    /// counters freezing, the binary exiting). Phase 4's plugin push
    /// implementation may introduce a richer error surface if the
    /// editor needs to disambiguate; this trait will widen at that
    /// point, not now.
    ///
    /// The snapshot itself is informational, not load-bearing for
    /// correctness. The render path, voice allocator, and transport do
    /// not depend on the GUI polling it.
    fn poll_snapshot(&self) -> Option<EngineSnapshot>;

    /// Handle to the atomic transport counters.
    fn transport_metrics(&self) -> &Arc<TransportMetrics>;

    /// Handle to the atomic input/MIDI counters.
    fn input_metrics(&self) -> &Arc<InputMetrics>;

    /// Handle to the atomic recording counters.
    fn recording_metrics(&self) -> &Arc<RecordingMetrics>;
}

/// Standalone implementation of [`SessionStateSource`] over a borrowed
/// [`RuntimeSession`].
///
/// The wrapper itself does no work:
/// [`SessionStateSource::poll_snapshot`] forwards directly to
/// [`RuntimeSession::request_snapshot`] (preserving its 500 ms timeout
/// contract), and the metrics accessors return references to the
/// `Arc<*Metrics>` fields already held by the session. The wrapper adds
/// no allocation, no clone, no extra channel.
///
/// The per-call cost is dominated by [`RuntimeSession::request_snapshot`],
/// which round-trips an `mpsc` message to the engine thread and
/// receives an [`EngineSnapshot`] whose body the engine allocates
/// (vectors of held notes, voice phases, tag strings). That cost is not
/// added here, but it is also not eliminated — it is the reason the
/// trait is rate-limited to the 75 ms `PERFORMANCE_UI_REFRESH` cadence
/// rather than polled per frame.
pub struct StandaloneSessionSource<'a> {
    session: &'a RuntimeSession,
}

impl<'a> StandaloneSessionSource<'a> {
    /// Wrap a [`RuntimeSession`] in a [`SessionStateSource`] view.
    pub fn new(session: &'a RuntimeSession) -> Self {
        Self { session }
    }
}

impl<'a> SessionStateSource for StandaloneSessionSource<'a> {
    fn poll_snapshot(&self) -> Option<EngineSnapshot> {
        self.session.request_snapshot().ok()
    }

    fn transport_metrics(&self) -> &Arc<TransportMetrics> {
        &self.session.transport_metrics
    }

    fn input_metrics(&self) -> &Arc<InputMetrics> {
        &self.session.input_metrics
    }

    fn recording_metrics(&self) -> &Arc<RecordingMetrics> {
        &self.session.recording_metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ConstantSource {
        transport: Arc<TransportMetrics>,
        input: Arc<InputMetrics>,
        recording: Arc<RecordingMetrics>,
    }

    impl ConstantSource {
        fn new() -> Self {
            Self {
                transport: Arc::new(TransportMetrics::default()),
                input: Arc::new(InputMetrics::default()),
                recording: Arc::new(RecordingMetrics::default()),
            }
        }

        fn from_handles(
            transport: Arc<TransportMetrics>,
            input: Arc<InputMetrics>,
            recording: Arc<RecordingMetrics>,
        ) -> Self {
            Self {
                transport,
                input,
                recording,
            }
        }
    }

    impl SessionStateSource for ConstantSource {
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

    #[test]
    fn trait_dispatches_dynamically() {
        let source: Box<dyn SessionStateSource> = Box::new(ConstantSource::new());
        assert!(source.poll_snapshot().is_none());
        assert_eq!(source.transport_metrics().snapshot().queued_frames, 0);
        assert_eq!(source.input_metrics().snapshot().midi_messages, 0);
        assert_eq!(source.recording_metrics().snapshot().frames_written, 0);
    }

    #[test]
    fn metrics_handles_are_arc_clones() {
        let source = ConstantSource::new();
        let transport_handle = Arc::clone(source.transport_metrics());
        transport_handle.set_queued_frames(2048);
        assert_eq!(source.transport_metrics().snapshot().queued_frames, 2048);
    }

    /// Locks in the `no clone, no eager refresh` contract from the
    /// [`StandaloneSessionSource`] doc comment: the accessors must
    /// return references to the *same* `Arc<*Metrics>` they were
    /// constructed with, not freshly-cloned handles. A regression that
    /// returned `&Arc::clone(&self.transport)` would still type-check
    /// against the trait but would break the shared-handle contract.
    ///
    /// We cannot easily construct a real [`RuntimeSession`] in a unit
    /// test (it depends on ALSA, the engine worker thread, MIDI). The
    /// test exercises the *shape* of the contract through a mock that
    /// mirrors the standalone wrapper.
    #[test]
    fn accessor_arcs_are_pointer_equal_to_constructed_handles() {
        let transport = Arc::new(TransportMetrics::default());
        let input = Arc::new(InputMetrics::default());
        let recording = Arc::new(RecordingMetrics::default());
        let source = ConstantSource::from_handles(
            Arc::clone(&transport),
            Arc::clone(&input),
            Arc::clone(&recording),
        );

        assert!(Arc::ptr_eq(source.transport_metrics(), &transport));
        assert!(Arc::ptr_eq(source.input_metrics(), &input));
        assert!(Arc::ptr_eq(source.recording_metrics(), &recording));
    }
}
