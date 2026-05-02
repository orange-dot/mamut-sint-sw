# BCS v1.1 Standalone Runtime Flag Evidence

Date: 2026-05-01

This slice makes the existing `mamut-engine` BCS smoke layer reachable through
`mamut-standalone` for listening. It is still disabled by default and still has
no MIDI map, patch schema, profile, or factory patch contract.

## Runtime Surface

Launch-time flags:

```bash
mamut-standalone dry-run --bcs-layer-scenario <scenario> [factory-name-or-path]
mamut-standalone play --bcs-layer-scenario <scenario> [factory-name-or-path]
```

Accepted scenario values:

- `stable-anchor`
- `edge-sweep`
- `subharmonic-pressure`
- `recovery-return`
- `off`

Interactive command during `play`:

```text
bcs <off|stable-anchor|edge-sweep|subharmonic-pressure|recovery-return>
```

Status line:

```text
bcs: mode=enabled scenario=<scenario> active=<scenario> sample_rate=<hz> max_state=<value> unsafe_events=<n> unsafe=<bool>
```

## Validation

Commands:

```bash
cargo fmt --all
cargo test -p mamut-standalone bcs -- --nocapture
cargo run -p mamut-standalone -- dry-run --bcs-layer-scenario subharmonic-pressure cathedral-bloom
cargo fmt --all --check
cargo test -p mamut-engine bcs_layer -- --nocapture
cargo test -p mamut-standalone
cargo build --release -p mamut-standalone
target/release/mamut-standalone dry-run --bcs-layer-scenario recovery-return ember-vault
```

Observed result:

```text
Standalone BCS focused tests: 3 passed; 0 failed
Engine BCS focused tests: 5 passed; 0 failed
mamut-standalone full tests: 46 passed; 0 failed
mamut-standalone release build: finished
Release dry-run status: bcs mode enabled, recovery-return active, unsafe_events=0, unsafe=false
```

## Listening Command

Use a real ALSA hw selector from `mamut-standalone list-audio`, then run:

```bash
target/release/mamut-standalone play --demo --audio-device hw:CARD,DEVICE --bcs-layer-scenario subharmonic-pressure cathedral-bloom
```

During playback, switch scenarios without restarting:

```text
bcs stable-anchor
bcs edge-sweep
bcs subharmonic-pressure
bcs recovery-return
bcs off
```

## Boundary

Changed runtime surface:

- standalone dry-run flag
- standalone play flag
- standalone interactive `bcs` command
- standalone status line

Unchanged:

- patch schema
- factory patches
- profiles
- MIDI mapping
- GUI controls
- ALSA transport architecture

Listening verdict: pending real playback.
