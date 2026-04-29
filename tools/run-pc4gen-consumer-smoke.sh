#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
pc4gen_root="${PC4GEN_ROOT:-/home/dev/work-base-20260421/workspace/systems/audio-runtime-lab/topics/37_pc4_synthetic_midi_capture/rust/pc4gen}"
derived_root="${PC4GEN_DERIVED:-/home/dev/work-base-20260421/synthetic-pc4-capture/derived}"
profile_path="${PC4GEN_PROFILE:-}"
scenario="${MAMUT_PC4GEN_SCENARIO:-mamut-epm1-smoke}"
seed="${MAMUT_PC4GEN_SEED:-20260426}"
duration="${MAMUT_PC4GEN_DURATION:-14s}"
mamut_timeout="${MAMUT_PC4GEN_MAMUT_TIMEOUT:-18s}"
client_name="${MAMUT_PC4GEN_CLIENT_NAME:-pc4gen-mamut-smoke}"
patch_name="${MAMUT_PC4GEN_PATCH:-molten-horizon}"
audio_device="${MAMUT_AUDIO_DEVICE:-}"
midi_channel="${MAMUT_MIDI_CHANNEL:-1}"
work_dir="${MAMUT_PC4GEN_WORK_DIR:-}"

usage() {
  cat <<'EOF'
usage: tools/run-pc4gen-consumer-smoke.sh [options]

Run the local live ALSA consumer smoke:
  pc4gen live -> virtual ALSA MIDI source -> mamut-standalone --trace-midi

options:
  --pc4gen-root <path>       pc4gen crate root
  --derived <path>           derived capture root used to build a pc4gen profile
  --profile <path>           use an existing pc4gen profile instead of building one
  --scenario <name>          pc4gen scenario (default: mamut-epm1-smoke)
  --seed <n>                 deterministic seed (default: 20260426)
  --duration <duration>      pc4gen live duration (default: 14s)
  --audio-device <selector>  ALSA playback selector for Mamut
  --midi-channel <1..16>     accepted MIDI channel for Mamut (default: 1)
  --patch <name-or-path>     Mamut patch/factory name (default: molten-horizon)
  --work-dir <path>          keep logs/profile in this directory
  -h, --help                 show this help

environment overrides:
  PC4GEN_ROOT
  PC4GEN_DERIVED
  PC4GEN_PROFILE
  MAMUT_AUDIO_DEVICE
  MAMUT_MIDI_CHANNEL
  MAMUT_PC4GEN_SCENARIO
  MAMUT_PC4GEN_SEED
  MAMUT_PC4GEN_DURATION
  MAMUT_PC4GEN_MAMUT_TIMEOUT
  MAMUT_PC4GEN_PATCH
  MAMUT_PC4GEN_WORK_DIR
EOF
}

detect_ag_audio_device() {
  local card_pattern
  local card_index

  card_pattern="${MAMUT_AG_CARD_PATTERN:-AG06AG03|AG06/AG03}"
  [[ -r /proc/asound/cards && -r /proc/asound/pcm ]] || return 1

  card_index="$(awk -v pattern="$card_pattern" '
    $0 ~ pattern {
      print $1
      exit
    }
  ' /proc/asound/cards)"
  [[ -n "${card_index:-}" ]] || return 1

  awk -v card="$card_index" '
    match($0, /^([0-9]+)-([0-9]+):/, parts) &&
    (parts[1] + 0) == (card + 0) &&
    $0 ~ /playback/ {
      printf "hw:%d,%d\n", parts[1], parts[2]
      exit 0
    }
    END { exit 1 }
  ' /proc/asound/pcm
}

while (($#)); do
  case "$1" in
    --pc4gen-root)
      [[ $# -ge 2 ]] || { echo "error: --pc4gen-root requires a value" >&2; exit 1; }
      pc4gen_root="$2"
      shift 2
      ;;
    --derived)
      [[ $# -ge 2 ]] || { echo "error: --derived requires a value" >&2; exit 1; }
      derived_root="$2"
      shift 2
      ;;
    --profile)
      [[ $# -ge 2 ]] || { echo "error: --profile requires a value" >&2; exit 1; }
      profile_path="$2"
      shift 2
      ;;
    --scenario)
      [[ $# -ge 2 ]] || { echo "error: --scenario requires a value" >&2; exit 1; }
      scenario="$2"
      shift 2
      ;;
    --seed)
      [[ $# -ge 2 ]] || { echo "error: --seed requires a value" >&2; exit 1; }
      seed="$2"
      shift 2
      ;;
    --duration)
      [[ $# -ge 2 ]] || { echo "error: --duration requires a value" >&2; exit 1; }
      duration="$2"
      shift 2
      ;;
    --audio-device)
      [[ $# -ge 2 ]] || { echo "error: --audio-device requires a value" >&2; exit 1; }
      audio_device="$2"
      shift 2
      ;;
    --midi-channel)
      [[ $# -ge 2 ]] || { echo "error: --midi-channel requires a value" >&2; exit 1; }
      midi_channel="$2"
      shift 2
      ;;
    --patch)
      [[ $# -ge 2 ]] || { echo "error: --patch requires a value" >&2; exit 1; }
      patch_name="$2"
      shift 2
      ;;
    --work-dir)
      [[ $# -ge 2 ]] || { echo "error: --work-dir requires a value" >&2; exit 1; }
      work_dir="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "error: unknown argument $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

if [[ -z "$audio_device" ]]; then
  audio_device="$(detect_ag_audio_device || true)"
fi

if [[ -z "$audio_device" ]]; then
  echo "error: no ALSA playback device selected; pass --audio-device hw:<card>,<device> or set MAMUT_AUDIO_DEVICE" >&2
  exit 1
fi

if [[ ! -e /dev/snd/seq ]]; then
  echo "error: ALSA sequencer is unavailable at /dev/snd/seq; load/enable snd_seq before running the live MIDI smoke" >&2
  exit 1
fi

if [[ -z "$work_dir" ]]; then
  work_dir="$(mktemp -d)"
else
  mkdir -p "$work_dir"
fi

pc4gen_pid=""
cleanup() {
  if [[ -n "$pc4gen_pid" ]] && kill -0 "$pc4gen_pid" 2>/dev/null; then
    kill "$pc4gen_pid" 2>/dev/null || true
    wait "$pc4gen_pid" 2>/dev/null || true
  fi
}
trap cleanup EXIT

pc4gen_bin="$pc4gen_root/target/debug/pc4gen"
mamut_bin="$repo_root/target/debug/mamut-standalone"
pc4gen_stdout="$work_dir/pc4gen.stdout.log"
pc4gen_stderr="$work_dir/pc4gen.stderr.log"
mamut_stdout="$work_dir/mamut.stdout.log"
mamut_stderr="$work_dir/mamut.stderr.log"
midi_ports_log="$work_dir/midi-ports.log"
metrics_out="$work_dir/pc4gen.metrics.json"

printf 'Building pc4gen\n'
cargo build --manifest-path "$pc4gen_root/Cargo.toml" --bin pc4gen

printf 'Building mamut-standalone\n'
cargo build --locked --manifest-path "$repo_root/Cargo.toml" -p mamut-standalone

if [[ -z "$profile_path" ]]; then
  profile_path="$work_dir/pc4gen-mamut-profile.json"
  printf 'Building pc4gen profile from %s\n' "$derived_root"
  "$pc4gen_bin" profile from-derived --derived "$derived_root" --out "$profile_path" >/dev/null
fi

"$pc4gen_bin" profile show --profile "$profile_path" --scenario "$scenario" >/dev/null

printf 'Starting pc4gen live source\n'
"$pc4gen_bin" live \
  --profile "$profile_path" \
  --scenario "$scenario" \
  --seed "$seed" \
  --duration "$duration" \
  --channel "$midi_channel" \
  --alsa-client-name "$client_name" \
  --metrics-out "$metrics_out" \
  >"$pc4gen_stdout" 2>"$pc4gen_stderr" &
pc4gen_pid="$!"

for _ in $(seq 1 80); do
  if ! kill -0 "$pc4gen_pid" 2>/dev/null; then
    echo "error: pc4gen exited before Mamut could connect" >&2
    cat "$pc4gen_stderr" >&2 || true
    exit 1
  fi
  if "$mamut_bin" list-midi >"$midi_ports_log" 2>&1 && grep -Fqi "$client_name" "$midi_ports_log"; then
    break
  fi
  sleep 0.1
done

if ! grep -Fqi "$client_name" "$midi_ports_log"; then
  echo "error: pc4gen MIDI source was not visible to Mamut" >&2
  cat "$midi_ports_log" >&2 || true
  exit 1
fi

printf 'Running Mamut consumer smoke\n'
mamut_status=0
(
  cd "$repo_root"
  timeout -s INT "$mamut_timeout" "$mamut_bin" play \
    --headless \
    --audio-device "$audio_device" \
    --midi-device "$client_name" \
    --midi-channel "$midi_channel" \
    --trace-midi \
    "$patch_name"
) >"$mamut_stdout" 2>"$mamut_stderr" || mamut_status=$?

case "$mamut_status" in
  0|124|130|143) ;;
  *)
    echo "error: mamut-standalone failed with status $mamut_status" >&2
    cat "$mamut_stderr" >&2 || true
    exit "$mamut_status"
    ;;
esac

require_trace() {
  local label="$1"
  local pattern="$2"
  if ! grep -Fq "$pattern" "$mamut_stderr"; then
    echo "error: missing MIDI trace for $label: $pattern" >&2
    echo "trace log: $mamut_stderr" >&2
    tail -80 "$mamut_stderr" >&2 || true
    exit 1
  fi
}

require_trace "note on" "note on note="
require_trace "note off" "note off note="
require_trace "Gravitacija macro" "macro Gravitacija"
require_trace "Bloom macro" "macro Bloom"
require_trace "Heat macro" "macro Heat"
require_trace "Ruin macro" "macro Ruin"
require_trace "Swarm macro" "macro Swarm"
require_trace "mod wheel" "mod wheel amount="
require_trace "sustain down" "sustain down=true"
require_trace "sustain up" "sustain down=false"
require_trace "aftertouch" "aftertouch pressure="
require_trace "pitch bend" "pitch bend semitones="

for slot in 0 1 2 3 4 5 6 7; do
  require_trace "program change $slot" "program change slot=$slot"
done

printf 'pc4gen -> Mamut consumer smoke passed\n'
printf '  profile: %s\n' "$profile_path"
printf '  logs: %s\n' "$work_dir"
