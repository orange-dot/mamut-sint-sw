use super::*;

pub(crate) fn event_matches_binding(event: &LastControlEvent, binding: &ControllerBinding) -> bool {
    event_is_recent(event)
        && matches!(
            event.kind,
            LastControlKind::ProfileCc(cc) | LastControlKind::LegacyCc(cc) if cc == binding.cc
        )
}

pub(crate) fn event_matches_legacy_cc(event: &LastControlEvent, cc: u8) -> bool {
    event_is_recent(event)
        && matches!(event.kind, LastControlKind::LegacyCc(event_cc) if event_cc == cc)
}

pub(crate) fn event_matches_program_change(event: &LastControlEvent, slot: u8) -> bool {
    event_is_recent(event)
        && matches!(event.kind, LastControlKind::ProgramChange(program) if program == slot)
}

pub(crate) fn event_is_recent(event: &LastControlEvent) -> bool {
    Instant::now() <= event.received_at + MIDI_ACTIVITY_FLASH
}

pub(crate) fn verdict_label(verdict: LastControlVerdict) -> &'static str {
    match verdict {
        LastControlVerdict::Accepted => "accepted",
        LastControlVerdict::Filtered => "filtered",
        LastControlVerdict::StartupSuppressed => "startup suppressed",
        LastControlVerdict::Reserved => "reserved",
        LastControlVerdict::ReleaseIgnored => "release ignored",
        LastControlVerdict::Ignored => "ignored",
    }
}
