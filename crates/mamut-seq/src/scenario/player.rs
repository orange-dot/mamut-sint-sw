//! Monotonic absolute-deadline scheduler with a jitter meter.
//!
//! Each event fires at `start + at_ms` computed from a single `Instant`, so
//! there is no cumulative drift. Wall-clock jitter (`actual - deadline`) is
//! measured and reported; it is never claimed away.

use std::time::{Duration, Instant};

use anyhow::Result;

use crate::port::VirtualPort;

use super::expand::AbsoluteEvent;

/// Measured send timing for a completed (or interrupted) playback.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JitterReport {
    pub events: usize,
    pub sent: usize,
    pub interrupted: bool,
    pub max_ms: f64,
    pub mean_ms: f64,
}

/// Poll cadence while waiting for the next deadline.
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Play `events` against `port`. `should_stop` is polled between deadlines;
/// when it returns true, playback halts and the caller cleans up (the
/// `VirtualPort` drop, or an explicit `panic`, clears held notes).
pub fn play(
    port: &mut VirtualPort,
    events: &[AbsoluteEvent],
    should_stop: &mut dyn FnMut() -> bool,
) -> Result<JitterReport> {
    let start = Instant::now();
    let mut max_ms = 0.0f64;
    let mut sum_ms = 0.0f64;
    let mut sent = 0usize;
    let mut interrupted = false;

    for event in events {
        let deadline = start + Duration::from_millis(event.at_ms);
        if wait_until(deadline, should_stop) {
            interrupted = true;
            break;
        }
        let jitter_ms = Instant::now()
            .saturating_duration_since(deadline)
            .as_secs_f64()
            * 1000.0;
        max_ms = max_ms.max(jitter_ms);
        sum_ms += jitter_ms;
        port.send_tracked(&event.bytes)?;
        sent += 1;
    }

    let mean_ms = if sent == 0 { 0.0 } else { sum_ms / sent as f64 };
    Ok(JitterReport {
        events: events.len(),
        sent,
        interrupted,
        max_ms,
        mean_ms,
    })
}

/// Sleep until `deadline`, polling `should_stop`. Returns true if interrupted.
fn wait_until(deadline: Instant, should_stop: &mut dyn FnMut() -> bool) -> bool {
    loop {
        if should_stop() {
            return true;
        }
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        std::thread::sleep((deadline - now).min(POLL_INTERVAL));
    }
}
