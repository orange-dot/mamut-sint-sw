use super::*;

pub fn run_demo_performance(tx: mpsc::Sender<EngineCommand>, thread_stop: Arc<AtomicBool>) {
    let notes = [36_u8, 43, 48, 55, 60, 67];
    let macro_cycle = [
        (MacroId::Bloom, 0.65),
        (MacroId::Heat, 0.58),
        (MacroId::Gravitacija, 0.52),
        (MacroId::Ruin, 0.38),
        (MacroId::Swarm, 0.44),
        (MacroId::Gravitacija, 0.74),
        (MacroId::Ruin, 0.62),
    ];

    let mut step = 0_usize;
    loop {
        if thread_stop.load(Ordering::Relaxed) {
            break;
        }

        let note = notes[step % notes.len()];
        let (macro_id, macro_value) = macro_cycle[step % macro_cycle.len()];
        let velocity = 0.62 + (step % 3) as f32 * 0.10;

        if tx
            .send(EngineCommand::Controller(ControllerEvent::Macro {
                id: macro_id,
                value: macro_value,
            }))
            .is_err()
        {
            break;
        }
        if tx
            .send(EngineCommand::Controller(ControllerEvent::ModWheel {
                amount: ((step % 5) as f32) / 5.0,
            }))
            .is_err()
        {
            break;
        }
        if tx
            .send(EngineCommand::Note(NoteEvent::NoteOn { note, velocity }))
            .is_err()
        {
            break;
        }
        if sleep_interruptibly(&thread_stop, Duration::from_millis(420)) {
            break;
        }
        if tx
            .send(EngineCommand::Controller(
                ControllerEvent::ChannelAftertouch {
                    pressure: 0.20 + ((step % 4) as f32) * 0.12,
                },
            ))
            .is_err()
        {
            break;
        }
        if sleep_interruptibly(&thread_stop, Duration::from_millis(360)) {
            break;
        }
        if tx
            .send(EngineCommand::Note(NoteEvent::NoteOff { note }))
            .is_err()
        {
            break;
        }
        if sleep_interruptibly(&thread_stop, Duration::from_millis(140)) {
            break;
        }
        step = step.wrapping_add(1);
    }
}

pub fn sleep_interruptibly(stop: &AtomicBool, duration: Duration) -> bool {
    let mut remaining = duration;
    while remaining > Duration::ZERO {
        if stop.load(Ordering::Relaxed) {
            return true;
        }
        let slice = remaining.min(Duration::from_millis(50));
        thread::sleep(slice);
        remaining = remaining.saturating_sub(slice);
    }
    stop.load(Ordering::Relaxed)
}
