use super::*;

pub(super) fn pc4_full_profile() -> ControllerProfile {
    controller_profile_from_toml(
        include_str!("../../../../profiles/pc4-full.toml"),
        Path::new("profiles/pc4-full.toml"),
    )
    .expect("pc4-full profile parses")
}

pub(super) fn test_engine_thread_state_with_patch_path(path: &Path) -> EngineThreadState {
    let patch = load_patch_from_path(path).expect("patch loads");
    let engine = Engine::new(
        EngineConfig {
            sample_rate_hz: ALSA_PLAYBACK_SAMPLE_RATE_HZ as f32,
            max_block_frames: 2_048,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine builds");
    let (_tx, rx) = mpsc::channel();
    let (producer, _consumer) = RingBuffer::<StereoFrame>::new(AUDIO_QUEUE_CAPACITY_FRAMES);
    let (scope_producer, _scope_consumer) =
        RingBuffer::<StereoFrame>::new(SCOPE_QUEUE_CAPACITY_FRAMES);
    EngineThreadState::new(
        engine,
        rx,
        producer,
        scope_producer,
        Arc::new(AtomicBool::new(false)),
        Arc::new(ArrayQueue::new(MIDI_INPUT_QUEUE_CAPACITY)),
        Arc::new(PriorityActions::default()),
        Arc::new(TransportMetrics::default()),
        Arc::new(InputMetrics::default()),
        Arc::new(RecordingMetrics::default()),
    )
}

pub(super) fn test_engine_thread_state() -> EngineThreadState {
    test_engine_thread_state_with_patch_path(&default_patch_path())
}

pub(super) fn test_engine_thread_state_for_patch(stem: &str) -> EngineThreadState {
    let path = resolve_patch_argument(Some(stem)).expect("factory patch resolves");
    test_engine_thread_state_with_patch_path(&path)
}

pub(super) fn test_snapshot() -> EngineSnapshot {
    let patch = load_patch_from_path(&default_patch_path()).expect("default patch loads");
    Engine::new(
        EngineConfig {
            sample_rate_hz: ALSA_PLAYBACK_SAMPLE_RATE_HZ as f32,
            max_block_frames: 2_048,
            voice_count: 6,
        },
        patch,
    )
    .expect("engine builds")
    .snapshot()
}
