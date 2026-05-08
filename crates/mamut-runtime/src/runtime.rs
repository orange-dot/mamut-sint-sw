use super::*;

pub fn build_audio_runtime(
    patch_path: &Path,
    audio_selector: Option<&str>,
    sample_rate_hz: u32,
    alsa_tuning: AlsaPlaybackTuning,
    gfm_layer_seed: Option<u64>,
    bcs_layer_scenario: Option<BcsScenario>,
    input_metrics: Arc<InputMetrics>,
    recording_metrics: Arc<RecordingMetrics>,
) -> Result<PreparedAudioRuntime> {
    let patch = load_patch_from_path(patch_path)?;
    validate_patch_v1(&patch).context("patch validation failed")?;
    let selected_device = select_alsa_output_device(audio_selector)?;
    let engine_sample_rate_hz = sample_rate_hz as f32;
    let channels = ALSA_PLAYBACK_CHANNELS;
    let bend_range = patch.performance_response.bend_range_semitones as f32;
    let patch_name = patch.meta.patch_name.clone();

    let (tx, rx) = mpsc::channel::<EngineCommand>();
    let transport_metrics = Arc::new(TransportMetrics::default());
    transport_metrics.record_write_request(alsa_tuning.period_frames);
    let midi_input_queue = Arc::new(ArrayQueue::new(MIDI_INPUT_QUEUE_CAPACITY));
    let priority_actions = Arc::new(PriorityActions::default());
    let mut engine = Engine::new(
        EngineConfig {
            sample_rate_hz: engine_sample_rate_hz,
            max_block_frames: 2_048,
            voice_count: 6,
        },
        patch,
    )?;
    engine.set_gfm_layer_mode(gfm_layer_mode_from_seed(gfm_layer_seed));
    engine.set_bcs_layer_mode(bcs_layer_mode_from_scenario(bcs_layer_scenario));
    let (producer, consumer) = RingBuffer::<StereoFrame>::new(AUDIO_QUEUE_CAPACITY_FRAMES);
    let worker = EngineWorker::new(spawn_engine_thread(
        engine,
        rx,
        producer,
        Arc::clone(&midi_input_queue),
        Arc::clone(&priority_actions),
        Arc::clone(&transport_metrics),
        input_metrics,
        recording_metrics,
    ));
    Ok(PreparedAudioRuntime {
        tx,
        worker,
        consumer,
        selected_device: selected_device.clone(),
        audio_selector: selected_device.selector.clone(),
        audio_device_name: selected_device.display_name(),
        sample_rate_hz,
        channels,
        alsa_tuning,
        bend_range,
        patch_name,
        midi_input_queue,
        priority_actions,
        transport_metrics,
    })
}

pub fn start_prepared_audio_runtime(prepared: PreparedAudioRuntime) -> Result<AudioRuntime> {
    let PreparedAudioRuntime {
        tx,
        worker,
        consumer,
        selected_device,
        audio_selector,
        audio_device_name,
        sample_rate_hz,
        channels,
        alsa_tuning,
        bend_range,
        patch_name,
        midi_input_queue,
        priority_actions,
        transport_metrics,
    } = prepared;

    let opened_playback =
        match open_alsa_playback_device(&selected_device, sample_rate_hz, alsa_tuning) {
            Ok(opened_playback) => opened_playback,
            Err(error) => {
                worker.shutdown(tx);
                return Err(error);
            }
        };
    transport_metrics.record_write_request(opened_playback.tuning.period_frames);
    let stream =
        AlsaPlaybackStream::spawn(opened_playback, consumer, Arc::clone(&transport_metrics));

    Ok(AudioRuntime {
        tx,
        worker,
        stream,
        audio_selector,
        audio_device_name,
        sample_rate_hz,
        channels,
        alsa_tuning,
        bend_range,
        patch_name,
        midi_input_queue,
        priority_actions,
        transport_metrics,
    })
}

pub fn print_runtime_help() {
    println!("runtime commands:");
    println!("  help                     show this command list");
    println!("  status                   show current patch, mode, macros, and activity");
    println!("  patches                  list factory patches");
    println!("  favorites                list the live-ready setlist slots");
    println!("  favorite <0..7>          load a setlist slot directly");
    println!("  patch <name-or-path>     load a factory patch or explicit TOML path");
    println!("  next                     load the next favorite patch");
    println!("  prev                     load the previous favorite patch");
    println!("  demo-patch               jump to the default opener patch");
    println!("  macro <name> <0..1>      set one public macro");
    println!("  panic                    clear all notes and live controller state");
    println!("  reset-controllers        clear bend/aftertouch/mod/sustain and macros");
    println!(
        "  bcs <off|stable-anchor|edge-sweep|subharmonic-pressure|recovery-return> set BCS layer"
    );
    println!("  record <seconds> [path]  record final Mamut stereo output to f32 WAV");
    println!("  record-stop              stop the active output recording");
    println!("  audio                    list ALSA hw playback outputs");
    println!(
        "  audio <index-or-hw:card,device> switch ALSA hw output (resets live performance state)"
    );
    println!("  --audio-device <selector> required ALSA selector at launch");
    println!("  --alsa-period-frames <n> ALSA period size in frames");
    println!("  --alsa-buffer-frames <n> ALSA buffer size in frames");
    println!("  --alsa-start-threshold-frames <n> ALSA start threshold in frames");
    println!("  midi                     list available MIDI inputs");
    println!("  midi <name-or-index>     switch to a MIDI input");
    println!("  --midi-channel <1..16>   accept MIDI only from one channel at launch");
    println!("  --controller-profile <path> load TOML controller bindings");
    println!("  --trace-midi             log incoming MIDI messages to stderr");
    println!("  --gfm-layer-seed <u64-or-0xHEX> enable the engine GFM layer at launch");
    println!("  --bcs-layer-scenario <scenario> select the engine BCS scenario mode at launch");
    println!("  demo                     switch the session to the demo performer");
    println!("  quit                     stop playback and exit");
}
