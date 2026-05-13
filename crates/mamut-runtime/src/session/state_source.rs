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

/// Owned `'static` view of the same subset of [`RuntimeSession`] state
/// as [`StandaloneSessionSource`], suitable for capture into a `'static`
/// closure (the `vizia` timer closure for the Phase 1 spike).
///
/// [`StandaloneSessionSource`] borrows the session and therefore cannot
/// be `Box<dyn SessionStateSource + 'static>`'d into a long-lived
/// callback. `StandaloneSessionHandle` solves that by *cloning* the
/// minimum surface the trait requires:
///
/// - a [`mpsc::Sender<EngineCommand>`] (`Sender` is `Clone` and the
///   send channel is the same one [`RuntimeSession::request_snapshot`]
///   uses);
/// - three `Arc<*Metrics>` clones — the atomic counters live on the
///   engine side and the handle holds reference-counted views.
///
/// The original [`RuntimeSession`] remains the owner of the engine
/// worker, the ALSA stream, the MIDI input, and the scope ring buffer.
/// This handle is intentionally narrow: it cannot drive playback,
/// switch patches, or drain scope frames. It is a *read-only consumer*
/// view, the same as the borrowed wrapper.
///
/// **Cost model.** Construction clones four reference-counted handles
/// (one `Sender`, three `Arc`s) — four atomic increments, no
/// allocation. Per-call cost on [`SessionStateSource::poll_snapshot`]
/// is identical to [`StandaloneSessionSource::poll_snapshot`]: one
/// `mpsc::channel()` allocation for the reply, one channel send to the
/// engine worker, one `recv_timeout` of up to 500 ms. The borrowed
/// wrapper attributes that allocation to
/// [`RuntimeSession::request_snapshot`] which it forwards to; the
/// owned handle inlines the same allocation in its own `impl`. The
/// per-call cost is the same in both cases.
///
/// **Lifecycle.** Dropping the handle decrements four reference counts
/// and does nothing else — it does not signal the engine worker, does
/// not stop the audio stream, does not close the command channel
/// (other senders, including the original session, keep it open). If
/// the original [`RuntimeSession`] is dropped while the handle is
/// alive, the engine worker shuts down; subsequent
/// [`poll_snapshot`](SessionStateSource::poll_snapshot) calls then
/// return `None` because the `send` fails (no receiver). The metrics
/// accessors keep returning the last-published counter values from the
/// `Arc<*Metrics>`, which is the right behavior for a GUI rendering
/// last-known-good state per ADR 0002.
///
/// **Auto-trait posture.** The handle is `Send` automatically: the
/// three `Arc<*Metrics>` clones are `Send + Sync`, and
/// [`mpsc::Sender<T>`] is `Send` when `T: Send` (it is here). The
/// handle is **not** `Sync`, because `mpsc::Sender` is famously
/// single-producer-cell — shared access from multiple threads must go
/// through `Sender::clone`, not `&Sender`. The trait itself imposes
/// neither bound; Phase 1 vizia runs the timer on the UI thread, so
/// `Sync` is not load-bearing and the missing auto-trait does not
/// constrain the spike.
pub struct StandaloneSessionHandle {
    tx: mpsc::Sender<EngineCommand>,
    transport_metrics: Arc<TransportMetrics>,
    input_metrics: Arc<InputMetrics>,
    recording_metrics: Arc<RecordingMetrics>,
}

impl StandaloneSessionHandle {
    /// Build a handle from a borrowed [`RuntimeSession`]. The session
    /// remains the owner; this clone is cheap (four atomic increments).
    pub fn from_session(session: &RuntimeSession) -> Self {
        Self {
            tx: session.tx.clone(),
            transport_metrics: Arc::clone(&session.transport_metrics),
            input_metrics: Arc::clone(&session.input_metrics),
            recording_metrics: Arc::clone(&session.recording_metrics),
        }
    }

    /// Build a handle directly from its four reference-counted parts.
    ///
    /// This constructor exists for tests in this crate that want to
    /// verify the handle's contract (pointer equality, channel-closed
    /// semantics) without spinning up a real [`RuntimeSession`].
    /// Production code uses [`Self::from_session`]. The visibility is
    /// deliberately `#[cfg(test)] pub(crate)` so the test-only
    /// constructor does not widen `mamut-runtime`'s public surface and
    /// is not present in release builds; downstream crates that need a
    /// mock should implement [`SessionStateSource`] directly.
    #[cfg(test)]
    pub(crate) fn new(
        tx: mpsc::Sender<EngineCommand>,
        transport_metrics: Arc<TransportMetrics>,
        input_metrics: Arc<InputMetrics>,
        recording_metrics: Arc<RecordingMetrics>,
    ) -> Self {
        Self {
            tx,
            transport_metrics,
            input_metrics,
            recording_metrics,
        }
    }
}

impl SessionStateSource for StandaloneSessionHandle {
    // Mirrors `RuntimeSession::request_snapshot` in
    // `crates/mamut-runtime/src/session/status.rs:122`. Any change to
    // the `EngineCommand::RequestSnapshot` reply protocol must update
    // both implementations in lockstep.
    fn poll_snapshot(&self) -> Option<EngineSnapshot> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(EngineCommand::RequestSnapshot(reply_tx))
            .ok()?;
        reply_rx.recv_timeout(Duration::from_millis(500)).ok()
    }

    fn transport_metrics(&self) -> &Arc<TransportMetrics> {
        &self.transport_metrics
    }

    fn input_metrics(&self) -> &Arc<InputMetrics> {
        &self.input_metrics
    }

    fn recording_metrics(&self) -> &Arc<RecordingMetrics> {
        &self.recording_metrics
    }
}

/// Error path for [`SessionCommandSink`] writes.
///
/// `panic` and `reset_controllers` cannot fail — they touch an
/// `Arc<PriorityActions>` and the atomic stores never error. The
/// channel-backed writes (`set_macro`, `set_direct_param`) fail only
/// when the engine worker has shut down and the [`mpsc::Sender::send`]
/// returns an error. The trait collapses that one failure mode into
/// a single variant; richer error surfaces are a Phase 4 concern.
///
/// Derives `Copy + Hash` so callers can record-and-rethrow or key a
/// one-shot toast by variant without per-error allocation. The
/// [`std::fmt::Display`] / [`std::error::Error`] impls match the
/// wording [`RuntimeSession::set_macro`] already uses for the same
/// failure (`crates/mamut-runtime/src/session/commands.rs:136`) so a
/// GUI toast can present the error uniformly whether it came through
/// the trait or through the underlying session method.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandError {
    /// The engine worker is no longer reachable; the command channel
    /// has no live receiver. Subsequent calls will return the same
    /// error until the GUI is shut down or the runtime is rebuilt.
    EngineUnavailable,
}

impl std::fmt::Display for CommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EngineUnavailable => f.write_str("audio runtime is no longer available"),
        }
    }
}

impl std::error::Error for CommandError {}

/// Read-only write surface paired with [`SessionStateSource`].
///
/// Per the ADR 0002 amendment (2026-05-13), this trait covers the
/// GUI's *write* surface in the Phase 1 spike: priority actions
/// (`panic`, `reset_controllers`) and controller-channel writes
/// (`set_macro`, `set_direct_param`). Slot switching is deliberately
/// not part of this trait — `RuntimeSession::switch_patch(path)` is
/// `&mut self` and rebuilds the audio runtime; Phase 1 routes
/// slot-grid clicks through an `AppEvent::RequestSlotSwitch(slot)`
/// consumed by `main.rs` of `crates/mamut-vizia`, which holds
/// `&mut RuntimeSession`. Phase 4 (plugin) will revisit.
///
/// **Transport-freeze posture.** This trait adds no new channels, no
/// new queues, and no new worker threads. It is a thin doctrine layer
/// over already-shipped command paths (`Arc<PriorityActions>` for
/// `panic`/`reset_controllers`, `mpsc::Sender<EngineCommand>` for
/// `set_macro`/`set_direct_param`). The freeze in
/// `docs/EPM1_TRANSPORT_FREEZE.md` is not reopened.
///
/// **Auto-trait posture.** The trait imposes no `Send`/`Sync` bound,
/// matching [`SessionStateSource`]. The owned
/// [`StandaloneCommandHandle`] inherits `Send` from
/// `mpsc::Sender + Arc` but is **not** `Sync` because of
/// `mpsc::Sender`'s single-producer-cell. The Vizia Phase 1 spike
/// runs all widget callbacks on the UI thread, so `Sync` is not
/// load-bearing.
///
/// **Clamping.** `set_macro` clamps to `[0.0, 1.0]` and
/// `set_direct_param` clamps to the parameter's declared range, as
/// `RuntimeSession::set_macro` and `RuntimeSession::set_direct_param`
/// already do. Implementations must not loosen this contract.
pub trait SessionCommandSink {
    /// Request that the engine stop all voices on the next render
    /// boundary. Routed through the priority-action path established
    /// in ADR 0001; never blocks, never fails.
    fn panic(&self);

    /// Request that the engine reset MIDI controller state on the
    /// next render boundary. Same priority-action path as
    /// [`Self::panic`]; never blocks, never fails.
    fn reset_controllers(&self);

    /// Send a macro change to the engine. Implementations clamp
    /// `value` to `[0.0, 1.0]`.
    fn set_macro(&self, id: MacroId, value: f32) -> Result<(), CommandError>;

    /// Send a direct parameter change to the engine. Implementations
    /// clamp `value` to the parameter's declared min/max.
    fn set_direct_param(&self, id: ParamId, value: f32) -> Result<(), CommandError>;
}

/// Standalone implementation of [`SessionCommandSink`] over a borrowed
/// [`RuntimeSession`]. Mirrors [`StandaloneSessionSource<'a>`].
///
/// All four methods forward directly to the matching
/// [`RuntimeSession`] surface; the wrapper adds no allocation, no
/// clone, no extra channel. Suitable for short synchronous use; cannot
/// be captured by a `'static` widget callback.
pub struct StandaloneCommandSink<'a> {
    session: &'a RuntimeSession,
}

impl<'a> StandaloneCommandSink<'a> {
    /// Wrap a [`RuntimeSession`] in a [`SessionCommandSink`] view.
    pub fn new(session: &'a RuntimeSession) -> Self {
        Self { session }
    }
}

// Bodies of `set_macro` and `set_direct_param` are intentionally
// duplicated between `StandaloneCommandSink<'a>` and
// `StandaloneCommandHandle`. The two impls hold different fields
// (`&'a RuntimeSession` vs owned `mpsc::Sender + Arc<PriorityActions>`)
// but the channel-send logic is identical; keeping the two side by
// side lets a future single-file reviewer verify they have not
// drifted without chasing a private helper. DRY-ing the two would
// save four lines at the cost of an extra indirection.
impl<'a> SessionCommandSink for StandaloneCommandSink<'a> {
    fn panic(&self) {
        self.session.priority_actions.request_panic();
    }

    fn reset_controllers(&self) {
        self.session.priority_actions.request_reset_controllers();
    }

    fn set_macro(&self, id: MacroId, value: f32) -> Result<(), CommandError> {
        self.session
            .tx
            .send(EngineCommand::Controller(ControllerEvent::Macro {
                id,
                value: value.clamp(0.0, 1.0),
            }))
            .map_err(|_| CommandError::EngineUnavailable)
    }

    fn set_direct_param(&self, id: ParamId, value: f32) -> Result<(), CommandError> {
        let spec = param_spec(id);
        self.session
            .tx
            .send(EngineCommand::Controller(ControllerEvent::DirectParam {
                id,
                value: value.clamp(spec.min, spec.max),
            }))
            .map_err(|_| CommandError::EngineUnavailable)
    }
}

/// Owned `'static` view paired with [`StandaloneSessionHandle`].
///
/// Clones the [`mpsc::Sender<EngineCommand>`] and the
/// `Arc<PriorityActions>` from the [`RuntimeSession`]. The original
/// session keeps owning the engine worker, the audio stream, the MIDI
/// input, and the scope ring buffer; this handle is a write-only
/// companion that can be captured into a `'static` widget callback.
///
/// **Cost model.** Construction clones two reference-counted handles
/// — one `Sender`, one `Arc<PriorityActions>` — two atomic increments,
/// no allocation. Per-call cost is identical to the matching
/// [`RuntimeSession`] method: `panic` and `reset_controllers` set one
/// `AtomicBool`; `set_macro` and `set_direct_param` issue one channel
/// send each (no allocation in steady state — the channel is bounded
/// by the engine worker, not by the GUI).
///
/// **Lifecycle.** Dropping the handle decrements two reference counts
/// and does nothing else. The `Sender` clone keeps the channel half
/// alive only if other senders also exist; when the
/// [`RuntimeSession`] is dropped, sends through this handle start
/// returning [`CommandError::EngineUnavailable`].
pub struct StandaloneCommandHandle {
    tx: mpsc::Sender<EngineCommand>,
    priority_actions: Arc<PriorityActions>,
}

impl StandaloneCommandHandle {
    /// Build a handle from a borrowed [`RuntimeSession`]. The session
    /// remains the owner; this clone is cheap (two atomic increments).
    pub fn from_session(session: &RuntimeSession) -> Self {
        Self {
            tx: session.tx.clone(),
            priority_actions: Arc::clone(&session.priority_actions),
        }
    }

    /// Build a handle directly from its two reference-counted parts.
    ///
    /// Mirrors [`StandaloneSessionHandle::new`]: a `#[cfg(test)]`
    /// constructor for tests that want to verify the contract without
    /// spinning up a real [`RuntimeSession`]. Production uses
    /// [`Self::from_session`]. Visibility is deliberately
    /// `#[cfg(test)] pub(crate)` so the constructor does not widen
    /// `mamut-runtime`'s public surface and is not present in release
    /// builds; downstream crates that need a mock should implement
    /// [`SessionCommandSink`] directly.
    #[cfg(test)]
    pub(crate) fn new(
        tx: mpsc::Sender<EngineCommand>,
        priority_actions: Arc<PriorityActions>,
    ) -> Self {
        Self {
            tx,
            priority_actions,
        }
    }
}

impl SessionCommandSink for StandaloneCommandHandle {
    fn panic(&self) {
        self.priority_actions.request_panic();
    }

    fn reset_controllers(&self) {
        self.priority_actions.request_reset_controllers();
    }

    fn set_macro(&self, id: MacroId, value: f32) -> Result<(), CommandError> {
        self.tx
            .send(EngineCommand::Controller(ControllerEvent::Macro {
                id,
                value: value.clamp(0.0, 1.0),
            }))
            .map_err(|_| CommandError::EngineUnavailable)
    }

    fn set_direct_param(&self, id: ParamId, value: f32) -> Result<(), CommandError> {
        let spec = param_spec(id);
        self.tx
            .send(EngineCommand::Controller(ControllerEvent::DirectParam {
                id,
                value: value.clamp(spec.min, spec.max),
            }))
            .map_err(|_| CommandError::EngineUnavailable)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::panic)]

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

    /// Compile-time check: [`StandaloneSessionHandle`] satisfies the
    /// `'static + Sized` bound, erases to `Box<dyn SessionStateSource +
    /// 'static>`, and is `Send` — the auto-traits the struct doc
    /// claims and that the GUI bridge depends on. If a future change
    /// adds a borrow or a non-`Send` field (e.g. `Rc`, `Cell`) to the
    /// handle, this test fails to compile.
    #[test]
    fn standalone_handle_can_be_box_dyn_static_and_send() {
        let (tx, _rx) = mpsc::channel();
        let handle = StandaloneSessionHandle::new(
            tx,
            Arc::new(TransportMetrics::default()),
            Arc::new(InputMetrics::default()),
            Arc::new(RecordingMetrics::default()),
        );
        let _erased: Box<dyn SessionStateSource + Send> = Box::new(handle);
    }

    #[test]
    fn standalone_handle_accessor_arcs_are_pointer_equal() {
        let (tx, _rx) = mpsc::channel();
        let transport = Arc::new(TransportMetrics::default());
        let input = Arc::new(InputMetrics::default());
        let recording = Arc::new(RecordingMetrics::default());
        let handle = StandaloneSessionHandle::new(
            tx,
            Arc::clone(&transport),
            Arc::clone(&input),
            Arc::clone(&recording),
        );

        assert!(Arc::ptr_eq(handle.transport_metrics(), &transport));
        assert!(Arc::ptr_eq(handle.input_metrics(), &input));
        assert!(Arc::ptr_eq(handle.recording_metrics(), &recording));
    }

    /// When the receiver side of the command channel is dropped, the
    /// send fails immediately and `poll_snapshot` collapses to `None`
    /// without waiting out the 500 ms timeout. This is the
    /// "engine-worker-is-gone" path the trait doc calls out.
    #[test]
    fn standalone_handle_poll_snapshot_returns_none_when_engine_channel_closed() {
        let (tx, rx) = mpsc::channel();
        let handle = StandaloneSessionHandle::new(
            tx,
            Arc::new(TransportMetrics::default()),
            Arc::new(InputMetrics::default()),
            Arc::new(RecordingMetrics::default()),
        );
        drop(rx);

        let start = std::time::Instant::now();
        let result = handle.poll_snapshot();
        let elapsed = start.elapsed();

        assert!(result.is_none());
        // The 500 ms recv_timeout must not fire — the send-failure path
        // returns immediately. Allow generous slack for slow CI.
        assert!(
            elapsed < Duration::from_millis(100),
            "expected fast-fail on closed channel, took {elapsed:?}"
        );
    }

    // -- SessionCommandSink + StandaloneCommandHandle ----------------

    /// Compile-time check: [`StandaloneCommandHandle`] erases to
    /// `Box<dyn SessionCommandSink + Send + 'static>`. If a future
    /// change adds a borrow or a non-`Send` field (e.g. `Rc`, `Cell`)
    /// to the handle, this test fails to compile.
    #[test]
    fn standalone_command_handle_can_be_box_dyn_static_and_send() {
        let (tx, _rx) = mpsc::channel();
        let handle = StandaloneCommandHandle::new(tx, Arc::new(PriorityActions::default()));
        let _erased: Box<dyn SessionCommandSink + Send> = Box::new(handle);
    }

    #[test]
    fn standalone_command_handle_priority_actions_arc_is_pointer_equal() {
        let (tx, _rx) = mpsc::channel();
        let priority = Arc::new(PriorityActions::default());
        let handle = StandaloneCommandHandle::new(tx, Arc::clone(&priority));

        handle.panic();
        // The Arc the handle holds is the same one we constructed, so
        // the priority-action state we observe through `priority` must
        // reflect the handle's call.
        assert_eq!(priority.take_action(), Some(PriorityAction::Panic));
    }

    #[test]
    fn standalone_command_handle_reset_controllers_propagates_through_arc() {
        let (tx, _rx) = mpsc::channel();
        let priority = Arc::new(PriorityActions::default());
        let handle = StandaloneCommandHandle::new(tx, Arc::clone(&priority));

        handle.reset_controllers();
        assert_eq!(
            priority.take_action(),
            Some(PriorityAction::ResetControllers)
        );
    }

    #[test]
    fn standalone_command_handle_set_macro_clamps_and_sends() {
        let (tx, rx) = mpsc::channel();
        let handle = StandaloneCommandHandle::new(tx, Arc::new(PriorityActions::default()));

        // Probe both clamp boundaries from one handle. The upper-bound
        // send (1.7 → 1.0) and lower-bound send (-0.3 → 0.0) share a
        // single `.clamp(0.0, 1.0)` call; testing both protects against
        // an asymmetric regression (e.g. someone replacing it with
        // `.min(1.0)`).
        for (input, label, expected) in [(1.7_f32, "upper", 1.0_f32), (-0.3_f32, "lower", 0.0_f32)]
        {
            handle
                .set_macro(MacroId::Gravitacija, input)
                .expect("send succeeds while rx alive");

            // Drain the channel and verify the clamp.
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(EngineCommand::Controller(ControllerEvent::Macro { id, value })) => {
                    assert_eq!(id, MacroId::Gravitacija);
                    assert!(
                        (value - expected).abs() < f32::EPSILON,
                        "{label} bound: value should be clamped to {expected}, got {value}"
                    );
                }
                other => panic!("expected Controller(Macro), got {other:?}"),
            }
        }
    }

    #[test]
    fn standalone_command_handle_set_macro_returns_engine_unavailable_when_channel_closed() {
        let (tx, rx) = mpsc::channel();
        let handle = StandaloneCommandHandle::new(tx, Arc::new(PriorityActions::default()));
        drop(rx);

        let result = handle.set_macro(MacroId::Heat, 0.5);
        assert_eq!(result, Err(CommandError::EngineUnavailable));
    }

    #[test]
    fn standalone_command_handle_set_direct_param_clamps_and_sends() {
        let (tx, rx) = mpsc::channel();
        let handle = StandaloneCommandHandle::new(tx, Arc::new(PriorityActions::default()));

        // OutputTrimDb is in the patch-shaped output stage; its spec
        // defines a finite range. Send a value well outside that range
        // and assert it lands clamped.
        let spec = param_spec(ParamId::FinalStageOutputTrimDb);
        let above = spec.max + 100.0;

        handle
            .set_direct_param(ParamId::FinalStageOutputTrimDb, above)
            .expect("send succeeds while rx alive");

        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(EngineCommand::Controller(ControllerEvent::DirectParam { id, value })) => {
                assert_eq!(id, ParamId::FinalStageOutputTrimDb);
                assert!(
                    (value - spec.max).abs() < f32::EPSILON,
                    "value should be clamped to spec.max ({}), got {value}",
                    spec.max
                );
            }
            other => panic!("expected Controller(DirectParam), got {other:?}"),
        }
    }
}
