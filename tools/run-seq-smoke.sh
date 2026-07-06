#!/usr/bin/env bash
set -euo pipefail

# Two-terminal mamut-seq smoke helper.
#
# mamut-seq is a pure MIDI SOURCE. The v1 flow is two-process by design: run
# Mamut in one terminal and mamut-seq in the other, with Mamut selecting the
# `mamut-seq` virtual port as its MIDI input. This script builds mamut-seq,
# validates a scenario, and prints the exact two-terminal quickstart. If an ALSA
# audio device is provided and --run is passed, it also drives a best-effort
# headless smoke end-to-end.

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scenario="${MAMUT_SEQ_SCENARIO:-gfm-gate-arm}"
channel="${MAMUT_SEQ_CHANNEL:-2}"
port_name="${MAMUT_SEQ_PORT_NAME:-mamut-seq}"
patch="${MAMUT_SEQ_PATCH:-cathedral-bloom}"
audio_device="${MAMUT_AUDIO_DEVICE:-}"
cargo_profile="${MAMUT_CARGO_PROFILE:-debug}"
autorun=0

usage() {
  cat <<'EOF'
usage: tools/run-seq-smoke.sh [options]

Build mamut-seq, validate a scenario, and print the two-terminal quickstart.
With --run and an ALSA audio device, also drive a best-effort headless smoke.

options:
  --scenario <name-or-path>  scenario under scenarios/ or a path (default: gfm-gate-arm)
  --channel <1..16>          MIDI channel (default: 2)
  --port-name <name>         virtual port name (default: mamut-seq)
  --patch <name-or-path>     Mamut factory patch for --run (default: cathedral-bloom)
  --audio-device <selector>  ALSA playback selector for Mamut (enables --run)
  --run                      actually drive the two-process smoke (needs --audio-device)
  -h, --help                 show this help

environment overrides:
  MAMUT_SEQ_SCENARIO  MAMUT_SEQ_CHANNEL  MAMUT_SEQ_PORT_NAME
  MAMUT_SEQ_PATCH     MAMUT_AUDIO_DEVICE MAMUT_CARGO_PROFILE
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --scenario) scenario="$2"; shift 2 ;;
    --channel) channel="$2"; shift 2 ;;
    --port-name) port_name="$2"; shift 2 ;;
    --patch) patch="$2"; shift 2 ;;
    --audio-device) audio_device="$2"; shift 2 ;;
    --run) autorun=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "unknown option: $1" >&2; usage >&2; exit 2 ;;
  esac
done

cd "$repo_root"

# Resolve the scenario to a path under scenarios/ when given a bare name.
if [ -f "$scenario" ]; then
  scenario_path="$scenario"
elif [ -f "scenarios/${scenario}.toml" ]; then
  scenario_path="scenarios/${scenario}.toml"
else
  echo "scenario not found: $scenario (looked for scenarios/${scenario}.toml)" >&2
  exit 1
fi

cargo_flags=(--locked -p mamut-seq)
[ "$cargo_profile" = "release" ] && cargo_flags+=(--release)

echo "building mamut-seq..."
cargo build "${cargo_flags[@]}"
seq_bin="target/${cargo_profile}/mamut-seq"

echo
echo "validating ${scenario_path}..."
# sed -n '1p' consumes the whole stream (prints only line 1) so validate never
# gets SIGPIPE from an early pipe close.
"$seq_bin" validate "$scenario_path" --channel "$channel" | sed -n '1p'

echo
echo "two-terminal quickstart:"
echo "  terminal 1 (Mamut receiver):"
echo "    cargo run --locked -p mamut-standalone -- play \\"
echo "      --audio-device <alsa-index-or-hw:card,device> \\"
echo "      --midi-device ${port_name} --midi-channel ${channel} \\"
echo "      --controller-profile profiles/pc4-full.toml --trace-midi ${patch}"
echo "  terminal 2 (mamut-seq source):"
echo "    cargo run --locked -p mamut-seq -- play ${scenario_path} \\"
echo "      --channel ${channel} --port-name ${port_name}"

if [ "$autorun" -ne 1 ]; then
  echo
  echo "(pass --run with --audio-device to drive this end-to-end here.)"
  exit 0
fi

if [ -z "$audio_device" ]; then
  echo "--run requires --audio-device (or MAMUT_AUDIO_DEVICE)" >&2
  exit 2
fi

echo
echo "best-effort headless smoke (Mamut may miss lead-in events before it connects):"
log_dir="$(mktemp -d)"
mamut_log="${log_dir}/mamut.log"

# Start the mamut-seq source first so the virtual port exists, then start Mamut
# selecting it. Mamut connects mid-stream; the scenario's lead-in wait covers
# most of the gap.
"$seq_bin" play "$scenario_path" --channel "$channel" --port-name "$port_name" &
seq_pid=$!

cargo run --locked -p mamut-standalone -- play \
  --audio-device "$audio_device" \
  --midi-device "$port_name" --midi-channel "$channel" \
  --controller-profile profiles/pc4-full.toml --trace-midi --headless "$patch" \
  >"$mamut_log" 2>&1 &
mamut_pid=$!

wait "$seq_pid" || true
kill "$mamut_pid" 2>/dev/null || true
wait "$mamut_pid" 2>/dev/null || true

echo "mamut trace-midi excerpt (${mamut_log}):"
grep -iE "cc3|note on|note off|program" "$mamut_log" | head -20 || true
echo "done."
