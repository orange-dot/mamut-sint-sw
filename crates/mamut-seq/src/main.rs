//! `mamut-seq` — laptop MIDI sequencer for EPM1 development.
//!
//! A pure MIDI *source*: it opens a virtual ALSA output port and plays
//! deterministic scenario files (or, in SET3-4, an interactive live TUI). It
//! makes no changes to `mamut-runtime`; the transport freeze is respected by
//! construction. See `docs/EPM1_BACKLOG_SET3_LAPTOP_MIDI_SEQUENCER.md`.

mod cli;
mod live;
mod midi;
mod port;
mod profile;
mod scenario;
#[cfg(test)]
mod tests;

use std::env;
use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use midir::{MidiInput, MidiOutput};

use cli::{CommonOpts, ParsedArgs, parse_args};
use midi::MidiChannel;
use port::VirtualPort;
use profile::Profile;
use scenario::ScenarioFile;

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        eprintln!();
        print_usage();
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("ports") => command_ports(),
        Some("validate") => command_validate(&parse_args(&args[1..])?),
        Some("play") => command_play(&parse_args(&args[1..])?),
        Some("live") => {
            let parsed = parse_args(&args[1..])?;
            live::run(&parsed.common, parsed.positional.as_deref().map(Path::new))
        }
        Some("help") | Some("--help") | Some("-h") => {
            print_usage();
            Ok(())
        }
        None => {
            print_usage();
            Ok(())
        }
        Some(other) => Err(anyhow!("unknown command `{other}`")),
    }
}

/// `mamut-seq ports` — list the MIDI ports the tool can see (a sanity aid).
fn command_ports() -> Result<()> {
    let mut lines = Vec::new();
    match MidiInput::new("mamut-seq") {
        Ok(input) => {
            lines.push("MIDI input ports:".to_string());
            push_port_lines(
                &mut lines,
                input.ports().iter().map(|port| input.port_name(port)),
            );
        }
        Err(error) => lines.push(format!("MIDI input unavailable: {error}")),
    }
    match MidiOutput::new("mamut-seq") {
        Ok(output) => {
            lines.push("MIDI output ports:".to_string());
            push_port_lines(
                &mut lines,
                output.ports().iter().map(|port| output.port_name(port)),
            );
        }
        Err(error) => lines.push(format!("MIDI output unavailable: {error}")),
    }
    write_lines(&lines)
}

fn push_port_lines(
    lines: &mut Vec<String>,
    names: impl Iterator<Item = Result<String, midir::PortInfoError>>,
) {
    let mut count = 0;
    for (index, name) in names.enumerate() {
        let name = name.unwrap_or_else(|_| "unknown-midi-port".to_string());
        lines.push(format!("  {index}: {name}"));
        count += 1;
    }
    if count == 0 {
        lines.push("  (none)".to_string());
    }
}

/// `mamut-seq validate <scenario.toml>` — parse, validate, and print the
/// expanded absolute event schedule without sending anything.
fn command_validate(parsed: &ParsedArgs) -> Result<()> {
    let scenario_path = parsed
        .positional
        .as_deref()
        .context("validate requires a scenario path")?;
    let scenario = scenario::load_from_path(Path::new(scenario_path))?;
    scenario::validate_scenario(&scenario).context("scenario validation failed")?;

    let profile = Profile::load(&parsed.common.profile_path)?;
    let channel = effective_channel(&scenario, &parsed.common)?;
    let events = scenario::expand(&scenario, &profile, channel)?;

    let mut lines = Vec::with_capacity(events.len() + 1);
    lines.push(format!(
        "valid: {scenario_path} ({}, {} events, channel {})",
        scenario.name,
        events.len(),
        channel.number()
    ));
    for event in &events {
        lines.push(format!(
            "{:>9} ms  {:<11}  {}",
            event.at_ms,
            format_bytes(&event.bytes),
            event.label
        ));
    }
    write_lines(&lines)
}

/// `mamut-seq play <scenario.toml>` — open the virtual port and send a scenario.
fn command_play(parsed: &ParsedArgs) -> Result<()> {
    let scenario_path = parsed
        .positional
        .as_deref()
        .context("play requires a scenario path")?;
    let scenario = scenario::load_from_path(Path::new(scenario_path))?;
    scenario::validate_scenario(&scenario).context("scenario validation failed")?;

    let profile = Profile::load(&parsed.common.profile_path)?;
    let channel = effective_channel(&scenario, &parsed.common)?;
    let events = scenario::expand(&scenario, &profile, channel)?;

    let mut virtual_port = VirtualPort::open(&parsed.common.port_name)?;
    virtual_port.configure_panic(profile.panic_cc(), profile.reset_controllers_cc());
    virtual_port.mark_channel(channel);
    print_line(&format!(
        "opened virtual MIDI port `{}` on channel {}",
        virtual_port.port_name(),
        channel.number()
    ))?;
    print_line(&format!(
        "playing `{}` ({} events)...",
        scenario.name,
        events.len()
    ))?;

    let report = if io::stdin().is_terminal() {
        let _raw = RawModeGuard::enable()?;
        print_line("press Ctrl-C or q to stop and panic-clear")?;
        scenario::play(&mut virtual_port, &events, &mut poll_interrupt)?
    } else {
        scenario::play(&mut virtual_port, &events, &mut || false)?
    };

    if report.interrupted {
        print_line(&format!(
            "interrupted after {}/{} events; panic-clearing",
            report.sent, report.events
        ))?;
    } else {
        print_line(&format!("done: {} events sent", report.sent))?;
    }
    print_line(&format!(
        "send jitter: max {:.3} ms, mean {:.3} ms",
        report.max_ms, report.mean_ms
    ))?;
    // Dropping `virtual_port` sends the panic sequence (note-offs + sustain
    // clear + profile/channel-mode resets).
    Ok(())
}

/// Write lines to stdout, exiting cleanly if the reader closes the pipe (Rust
/// ignores SIGPIPE, so an un-handled `EPIPE` would otherwise panic in `println!`).
fn write_lines(lines: &[String]) -> Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    for line in lines {
        match writeln!(out, "{line}") {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::BrokenPipe => return Ok(()),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

/// Write a single line, treating a broken pipe as a clean stop.
fn print_line(line: &str) -> Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();
    match writeln!(out, "{line}") {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn effective_channel(scenario: &ScenarioFile, common: &CommonOpts) -> Result<MidiChannel> {
    let number = scenario.channel.unwrap_or(common.channel);
    MidiChannel::new(number).with_context(|| format!("channel {number} out of range 1..16"))
}

fn format_bytes(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Non-blocking poll for a quit key while the terminal is in raw mode. Ctrl-C
/// arrives as a key event in raw mode, so we treat it (and `q`/Esc) as quit.
fn poll_interrupt() -> bool {
    match event::poll(Duration::ZERO) {
        Ok(true) => matches!(event::read(), Ok(Event::Key(key)) if is_quit_key(&key)),
        _ => false,
    }
}

fn is_quit_key(key: &KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char('q') | KeyCode::Esc)
        || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL))
}

/// RAII guard: enables terminal raw mode and disables it on drop.
struct RawModeGuard;

impl RawModeGuard {
    fn enable() -> Result<Self> {
        enable_raw_mode().map_err(|error| anyhow!("failed to enable raw mode: {error}"))?;
        Ok(Self)
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
    }
}

fn print_usage() {
    eprintln!(
        "usage:
  mamut-seq ports
  mamut-seq validate [--profile <path>] [--channel <1..16>] <scenario.toml>
  mamut-seq play [--profile <path>] [--channel <1..16>] [--port-name <name>] <scenario.toml>
  mamut-seq live [--profile <path>] [--channel <1..16>] [--port-name <name>] [--gate-ms <ms>] [scenario-to-fire]

defaults:
  --channel 2                       (mioXM/desktop rig convention)
  --port-name mamut-seq
  --profile profiles/pc4-full.toml

mamut-seq is a pure MIDI source. Run Mamut in one terminal and mamut-seq in
another; select the `mamut-seq` port on the Mamut side."
    );
}
