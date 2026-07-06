//! Hand-rolled argument parsing, mirroring the `mamut-standalone` idiom
//! (`match args.first()` dispatch + a `while index < args.len()` option loop,
//! `anyhow` errors, no `clap`).

use std::path::PathBuf;

use anyhow::{Context, Result, anyhow};

/// Default MIDI channel — the locked mioXM/desktop rig convention.
pub const DEFAULT_CHANNEL: u8 = 2;
/// Default virtual port name.
pub const DEFAULT_PORT_NAME: &str = "mamut-seq";
/// Default controller profile, relative to the repo root.
pub const DEFAULT_PROFILE_PATH: &str = "profiles/pc4-full.toml";

/// Default gate length (ms) for `live` note keys when key release is
/// unavailable.
pub const DEFAULT_GATE_MS: u64 = 150;

/// Flags shared by every subcommand that talks MIDI.
#[derive(Debug, Clone)]
pub struct CommonOpts {
    pub channel: u8,
    pub port_name: String,
    pub profile_path: PathBuf,
    /// `live` only: gate length for note keys without key-release events.
    pub gate_ms: u64,
}

impl Default for CommonOpts {
    fn default() -> Self {
        Self {
            channel: DEFAULT_CHANNEL,
            port_name: DEFAULT_PORT_NAME.to_string(),
            profile_path: PathBuf::from(DEFAULT_PROFILE_PATH),
            gate_ms: DEFAULT_GATE_MS,
        }
    }
}

/// Parsed common flags plus an optional single positional argument.
#[derive(Debug, Clone)]
pub struct ParsedArgs {
    pub common: CommonOpts,
    pub positional: Option<String>,
}

/// Parse `--channel`/`--port-name`/`--profile` and at most one positional arg.
pub fn parse_args(args: &[String]) -> Result<ParsedArgs> {
    let mut common = CommonOpts::default();
    let mut positional: Option<String> = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--channel" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --channel")?;
                common.channel = parse_midi_channel(value)?;
                index += 2;
            }
            "--port-name" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --port-name")?;
                common.port_name = value.clone();
                index += 2;
            }
            "--profile" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --profile")?;
                common.profile_path = PathBuf::from(value);
                index += 2;
            }
            "--gate-ms" => {
                let value = args
                    .get(index + 1)
                    .context("missing value after --gate-ms")?;
                common.gate_ms = value
                    .parse::<u64>()
                    .ok()
                    .filter(|ms| *ms > 0)
                    .with_context(|| {
                        format!("invalid --gate-ms `{value}`; expected a positive integer")
                    })?;
                index += 2;
            }
            option if option.starts_with("--") => {
                return Err(anyhow!("unknown option `{option}`"));
            }
            value if positional.is_none() => {
                positional = Some(value.to_string());
                index += 1;
            }
            extra => {
                return Err(anyhow!(
                    "unexpected extra argument `{extra}`; pass at most one scenario path"
                ));
            }
        }
    }

    Ok(ParsedArgs { common, positional })
}

/// Validate a `1..=16` MIDI channel (mirrors `mamut-runtime`'s validator).
pub fn parse_midi_channel(value: &str) -> Result<u8> {
    let channel = value
        .parse::<u8>()
        .with_context(|| format!("invalid MIDI channel `{value}`"))?;
    if (1..=16).contains(&channel) {
        Ok(channel)
    } else {
        Err(anyhow!(
            "invalid MIDI channel `{value}`; expected a value in the range 1..16"
        ))
    }
}
