use super::*;

#[test]
fn float_stereo_wav_writer_patches_header_sizes() {
    let path = std::env::temp_dir().join(format!(
        "mamut-test-{}.wav",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time after unix epoch")
            .as_nanos()
    ));
    {
        let mut writer = FloatStereoWavWriter::create(&path, 96_000).expect("writer creates");
        writer.write_frame([0.25, -0.25]).expect("frame writes");
        writer.write_frame([0.5, -0.5]).expect("frame writes");
        assert_eq!(writer.finalize().expect("writer finalizes"), 2);
    }
    let bytes = fs::read(&path).expect("wav file reads");
    let _ = fs::remove_file(&path);
    assert_eq!(&bytes[0..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(&bytes[12..16], b"fmt ");
    assert_eq!(u16::from_le_bytes([bytes[20], bytes[21]]), 3);
    assert_eq!(u16::from_le_bytes([bytes[22], bytes[23]]), 2);
    assert_eq!(
        u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]),
        96_000
    );
    assert_eq!(
        u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]),
        96_000 * 2 * std::mem::size_of::<f32>() as u32
    );
    assert_eq!(
        u16::from_le_bytes([bytes[32], bytes[33]]),
        (2 * std::mem::size_of::<f32>()) as u16
    );
    assert_eq!(u16::from_le_bytes([bytes[34], bytes[35]]), 32);
    assert_eq!(&bytes[36..40], b"data");
    assert_eq!(
        u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]),
        16
    );
    assert_eq!(bytes.len(), 44 + 16);
}

#[test]
fn factory_bank_has_productized_patch_set() {
    let entries = factory_patch_entries().expect("factory entries load");
    assert!(entries.len() >= 8);
    assert!(entries.iter().any(|entry| entry.favorite));
}

#[test]
fn favorite_navigation_wraps_across_live_set() {
    let next = adjacent_live_patch(Path::new("patches/factory/glass-tide.toml"), 1)
        .expect("favorite next resolves");
    assert!(next.ends_with("patches/factory/molten-horizon.toml"));

    let prev = adjacent_live_patch(Path::new("patches/factory/molten-horizon.toml"), -1)
        .expect("favorite prev resolves");
    assert!(prev.ends_with("patches/factory/glass-tide.toml"));
}

#[test]
fn live_set_slots_follow_locked_pc4_order() {
    let live_set = live_patch_entries().expect("live set loads");
    assert_eq!(live_set.len(), 8);
    assert_eq!(live_set[0].stem, "molten-horizon");
    assert_eq!(live_set[1].stem, "cathedral-bloom");
    assert_eq!(live_set[7].stem, "glass-tide");
}

#[test]
fn snapshot_requests_are_one_shot_and_do_not_drain_events() {
    let mut state = test_engine_thread_state();
    let (reply_tx, reply_rx) = mpsc::channel();
    state.snapshot_requests.push(reply_tx);
    state.note_events.push(Scheduled {
        frame_offset: 0,
        event: NoteEvent::NoteOn {
            note: 60,
            velocity: 1.0,
        },
    });
    state.controller_events.push(Scheduled {
        frame_offset: 0,
        event: ControllerEvent::Macro {
            id: MacroId::Bloom,
            value: 0.5,
        },
    });

    state.flush_snapshot_requests();

    assert!(reply_rx.recv_timeout(Duration::from_millis(50)).is_ok());
    assert!(reply_rx.try_recv().is_err());
    assert!(state.snapshot_requests.is_empty());
    assert_eq!(state.note_events.len(), 1);
    assert_eq!(state.controller_events.len(), 1);
}
