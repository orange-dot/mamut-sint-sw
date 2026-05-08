use super::*;

#[test]
fn load_patch_resets_runtime_state() {
    let mut engine = fixture_engine();
    let mut left = [0.0_f32; 128];
    let mut right = [0.0_f32; 128];

    engine.process_block(ProcessBlock {
        frame_count: 128,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.9,
            },
        }],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::Macro {
                id: MacroId::Gravitacija,
                value: 0.92,
            },
        }],
        macro_state: None,
        output: Some(StereoBlockMut::new(&mut left, &mut right)),
    });
    assert!(engine.snapshot().active_voice_count > 0);

    let replacement = load_patch_toml(include_str!("../../../../patches/factory/ember-vault.toml"))
        .expect("replacement patch parses");
    engine
        .load_patch(replacement)
        .expect("replacement patch loads");

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.patch_name, "Ember Vault");
    assert_eq!(snapshot.active_voice_count, 0);
    assert!(!snapshot.sustain_down);
    assert!(snapshot.peak_output.abs() <= f32::EPSILON);
}

#[test]
fn panic_clears_notes_and_controller_state() {
    let mut engine = fixture_engine();

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.95,
            },
        }],
        controller_events: &[
            Scheduled {
                frame_offset: 0,
                event: ControllerEvent::Sustain { down: true },
            },
            Scheduled {
                frame_offset: 1,
                event: ControllerEvent::ModWheel { amount: 0.8 },
            },
            Scheduled {
                frame_offset: 2,
                event: ControllerEvent::ChannelAftertouch { pressure: 0.7 },
            },
            Scheduled {
                frame_offset: 3,
                event: ControllerEvent::GfmLayerAmount { amount: 0.25 },
            },
        ],
        macro_state: None,
        output: None,
    });

    engine.panic();

    let snapshot = engine.snapshot();
    assert_eq!(snapshot.active_voice_count, 0);
    assert!(snapshot.held_notes.is_empty());
    assert!(!snapshot.sustain_down);
    assert_eq!(snapshot.gfm_layer.amount, 0.0);
    assert_eq!(snapshot.gfm_layer.pressure, 0.0);
    assert_eq!(snapshot.gfm_layer.effective_amount, 0.0);
    assert!(snapshot.live_macros == MacroState::from_defaults(&engine.patch.macros));
    assert!(!snapshot.clip_detected);
}

#[test]
fn panic_clears_patch_switch_mute_window() {
    let mut engine = fixture_engine();
    let replacement = load_patch_toml(include_str!("../../../../patches/factory/ember-vault.toml"))
        .expect("replacement patch parses");
    engine
        .load_patch(replacement)
        .expect("replacement patch loads");

    let (left_before, right_before) = engine.render_frame();
    assert_eq!((left_before, right_before), (0.0, 0.0));

    engine.panic();

    let (left_after, right_after) = engine.render_frame();
    assert!(left_after.is_finite());
    assert!(right_after.is_finite());
}

#[test]
fn reset_controllers_releases_sustain_state_but_keeps_held_voice() {
    let mut engine = fixture_engine();

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 0.9,
            },
        }],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::Sustain { down: true },
        }],
        macro_state: None,
        output: None,
    });
    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOff { note: 60 },
        }],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::Macro {
                id: MacroId::Ruin,
                value: 0.85,
            },
        }],
        macro_state: None,
        output: None,
    });

    assert!(engine.snapshot().sustain_down);
    engine.reset_controllers();

    let snapshot = engine.snapshot();
    assert!(!snapshot.sustain_down);
    assert!(snapshot.held_notes.contains(&60));
    assert!(snapshot.live_macros == MacroState::from_defaults(&engine.patch.macros));
}
