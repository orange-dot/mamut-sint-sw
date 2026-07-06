//! Interactive live TUI mode (`mamut-seq live`, Backlog SET3-4).
//!
//! A thin computer-keyboard instrument for ad-hoc sound checking without walking
//! to the hardware — it replaces the *dev-time* role of the PC4 keybed, not its
//! stage role. Note handling is event-driven with a bounded tick (no busy loop),
//! and quit/panic always leave the instrument clean via the port's `Drop` guard.

mod render;
mod state;

use std::io::{self, IsTerminal};
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};
use crossterm::cursor::{Hide, Show};
use crossterm::event::{
    self, Event, KeyboardEnhancementFlags, PopKeyboardEnhancementFlags,
    PushKeyboardEnhancementFlags,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
    supports_keyboard_enhancement,
};
use ratatui::Terminal;
use ratatui::backend::{Backend, CrosstermBackend};

use crate::cli::CommonOpts;
use crate::midi::MidiChannel;
use crate::port::VirtualPort;
use crate::profile::Profile;
use crate::scenario;

use state::{FiredScenario, LiveState};

/// Poll cadence: bounds CPU (poll blocks up to TICK) while keeping the
/// aftertouch ramp and gate note-offs smooth.
const TICK: Duration = Duration::from_millis(25);

/// Run the live TUI. `fire` optionally preloads a scenario the `f` key triggers.
pub fn run(common: &CommonOpts, fire: Option<&Path>) -> Result<()> {
    if !io::stdin().is_terminal() {
        return Err(anyhow!("live mode requires an interactive terminal"));
    }

    let profile = Profile::load(&common.profile_path)?;
    let channel = MidiChannel::new(common.channel)
        .ok_or_else(|| anyhow!("channel {} out of range 1..16", common.channel))?;

    let fired = match fire {
        Some(path) => {
            let scenario = scenario::load_from_path(path)?;
            scenario::validate_scenario(&scenario)?;
            // Honor the scenario's own channel override (consistent with `play`),
            // falling back to the CLI channel.
            let fire_number = scenario.channel.unwrap_or(common.channel);
            let fire_channel = MidiChannel::new(fire_number)
                .ok_or_else(|| anyhow!("scenario channel {fire_number} out of range 1..16"))?;
            let events = scenario::expand(&scenario, &profile, fire_channel)?;
            Some(FiredScenario {
                name: scenario.name,
                events,
            })
        }
        None => None,
    };

    // Open the port before the terminal guard so it drops last: the terminal is
    // restored first, then the port's Drop sends the panic sequence.
    let mut port = VirtualPort::open(&common.port_name)?;
    port.configure_panic(profile.panic_cc(), profile.reset_controllers_cc());
    port.mark_channel(channel);

    let guard = TerminalGuard::enter()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut live = LiveState::new(
        channel,
        &common.port_name,
        &profile,
        common.gate_ms,
        guard.release_available,
        fired,
        Instant::now(),
    );

    run_loop(&mut terminal, &mut port, &mut live)
}

fn run_loop<B>(
    terminal: &mut Terminal<B>,
    port: &mut VirtualPort,
    live: &mut LiveState,
) -> Result<()>
where
    B: Backend,
    B::Error: Send + Sync + 'static,
{
    loop {
        live.advance(port, Instant::now())?;
        if live.take_dirty() {
            terminal.draw(|frame| render::draw(frame, live))?;
        }
        if event::poll(TICK)? {
            if let Event::Key(key) = event::read()? {
                if live.handle_key(port, key)? {
                    break;
                }
            }
        }
    }
    Ok(())
}

/// RAII terminal guard: raw mode + alternate screen, plus kitty
/// keyboard-enhancement flags when the terminal supports them (for true
/// press/release). Restores everything on drop.
struct TerminalGuard {
    release_available: bool,
}

impl TerminalGuard {
    fn enter() -> Result<Self> {
        enable_raw_mode()?;
        // If any subsequent setup step fails, roll back so we never leave raw
        // mode / the alternate screen enabled without a live guard to restore it.
        match Self::setup() {
            Ok(release_available) => Ok(Self { release_available }),
            Err(error) => {
                let mut stdout = io::stdout();
                let _ = execute!(stdout, Show, LeaveAlternateScreen);
                let _ = disable_raw_mode();
                Err(error)
            }
        }
    }

    fn setup() -> Result<bool> {
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, Hide)?;
        let release_available = supports_keyboard_enhancement().unwrap_or(false);
        if release_available {
            execute!(
                stdout,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES)
            )?;
        }
        Ok(release_available)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        if self.release_available {
            let _ = execute!(stdout, PopKeyboardEnhancementFlags);
        }
        let _ = execute!(stdout, Show, LeaveAlternateScreen);
        let _ = disable_raw_mode();
    }
}
