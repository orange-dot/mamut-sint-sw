use super::*;

#[test]
fn allocator_prefers_idle_then_released_then_oldest_active() {
    let mut engine = fixture_engine();

    for note in 60..66 {
        engine.process_block(ProcessBlock {
            frame_count: 64,
            note_events: &[Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note,
                    velocity: 0.8,
                },
            }],
            controller_events: &[],
            macro_state: None,
            output: None,
        });
    }

    let full_snapshot = engine.snapshot();
    assert_eq!(full_snapshot.active_voice_count, 6);
    assert_eq!(full_snapshot.voices[0].note, Some(60));

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOff { note: 60 },
            },
            Scheduled {
                frame_offset: 1,
                event: NoteEvent::NoteOn {
                    note: 72,
                    velocity: 0.8,
                },
            },
        ],
        controller_events: &[],
        macro_state: None,
        output: None,
    });

    let released_reused = engine.snapshot();
    assert!(released_reused.held_notes.contains(&72));

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 73,
                velocity: 0.8,
            },
        }],
        controller_events: &[],
        macro_state: None,
        output: None,
    });

    let stolen = engine.snapshot();
    assert!(stolen.held_notes.contains(&73));
    assert!(!stolen.held_notes.contains(&61));
}

#[test]
fn note_off_prefers_held_voice_for_repeated_pitch() {
    let mut engine = fixture_engine();

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[
            Scheduled {
                frame_offset: 0,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.8,
                },
            },
            Scheduled {
                frame_offset: 1,
                event: NoteEvent::NoteOff { note: 60 },
            },
            Scheduled {
                frame_offset: 2,
                event: NoteEvent::NoteOn {
                    note: 60,
                    velocity: 0.7,
                },
            },
            Scheduled {
                frame_offset: 3,
                event: NoteEvent::NoteOff { note: 60 },
            },
        ],
        controller_events: &[],
        macro_state: None,
        output: None,
    });

    let snapshot = engine.snapshot();
    assert!(
        snapshot
            .voices
            .iter()
            .filter(|voice| voice.note == Some(60))
            .all(|voice| voice.phase != VoicePhase::Held)
    );
}

#[test]
fn sustain_state_transitions_are_stable() {
    let mut engine = fixture_engine();

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[Scheduled {
            frame_offset: 0,
            event: NoteEvent::NoteOn {
                note: 60,
                velocity: 1.0,
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
        controller_events: &[],
        macro_state: None,
        output: None,
    });

    let sustained = engine.snapshot();
    assert_eq!(sustained.voices[0].phase, VoicePhase::SustainedReleased);

    engine.process_block(ProcessBlock {
        frame_count: 64,
        note_events: &[],
        controller_events: &[Scheduled {
            frame_offset: 0,
            event: ControllerEvent::Sustain { down: false },
        }],
        macro_state: None,
        output: None,
    });
    assert_eq!(engine.snapshot().voices[0].phase, VoicePhase::Released);
}
