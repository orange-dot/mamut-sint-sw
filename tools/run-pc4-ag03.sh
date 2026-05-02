#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
patch_name="molten-horizon"
audio_device="${MAMUT_AUDIO_DEVICE:-}"
alsa_period_frames="${MAMUT_ALSA_PERIOD_FRAMES:-}"
alsa_buffer_frames="${MAMUT_ALSA_BUFFER_FRAMES:-}"
alsa_start_threshold_frames="${MAMUT_ALSA_START_THRESHOLD_FRAMES:-}"
midi_device="${MAMUT_MIDI_DEVICE:-mioXM DIN 1}"
midi_channel="${MAMUT_MIDI_CHANNEL:-1}"
controller_profile="${MAMUT_CONTROLLER_PROFILE:-$repo_root/profiles/pc4-full.toml}"
capture_dir="${MAMUT_CAPTURE_DIR:-$(cd "$repo_root/../../.." && pwd)/audio-captures}"
headless=1
demo=0
trace_midi=0
cargo_profile="${MAMUT_CARGO_PROFILE:-debug}"

usage() {
  cat <<'EOF'
usage: tools/run-pc4-ag03.sh [options] [factory-name-or-path]

Start the EPM1 standalone runtime with PC4 + mioXM + AG06/AG03 defaults.

options:
  --audio-device <alsa-index-or-hw:card,device>
                                  Override ALSA playback device
  --alsa-period-frames <n>        Override ALSA period size
  --alsa-buffer-frames <n>        Override ALSA buffer size
  --alsa-start-threshold-frames <n>
                                  Override ALSA start threshold
  --midi-device <name-or-index>   Override MIDI input device
  --midi-channel <1..16>          Accept MIDI only from one channel (default: 1)
  --controller-profile <path>     Override TOML controller profile
  --trace-midi                    Log incoming MIDI messages to stderr
  --demo                          Start with the built-in demo performer
  --windowed                      Prefer the egui performance window
  --headless                      Force the terminal runtime surface (default)
  --debug                         Run the debug cargo target (default)
  --release                       Run the release cargo target
  -h, --help                      Show this help

environment overrides:
  MAMUT_AUDIO_DEVICE              Optional explicit ALSA output device
  MAMUT_ALSA_PERIOD_FRAMES        Optional ALSA period size
  MAMUT_ALSA_BUFFER_FRAMES        Optional ALSA buffer size
  MAMUT_ALSA_START_THRESHOLD_FRAMES
                                  Optional ALSA start threshold
  MAMUT_MIDI_DEVICE               Default: mioXM DIN 1
  MAMUT_MIDI_CHANNEL              Default: 1
  MAMUT_CONTROLLER_PROFILE        Default: profiles/pc4-full.toml
  MAMUT_CAPTURE_DIR               Default: <lab-root>/audio-captures
  MAMUT_TRACE_MIDI                Set to 1 to enable MIDI tracing
  MAMUT_CARGO_PROFILE             debug or release (default: debug)
  MAMUT_AG_CARD_PATTERN           Regex used to find AG06/AG03 in /proc/asound/cards

examples:
  tools/run-pc4-ag03.sh
  tools/run-pc4-ag03.sh molten-horizon
  tools/run-pc4-ag03.sh --windowed gravity-wake
  tools/run-pc4-ag03.sh --midi-channel 1 molten-horizon
  tools/run-pc4-ag03.sh --trace-midi molten-horizon
  tools/run-pc4-ag03.sh --release --windowed --trace-midi molten-horizon
  tools/run-pc4-ag03.sh --audio-device hw:<card>,<device> molten-horizon
  tools/run-pc4-ag03.sh --audio-device 3 --alsa-period-frames 256 molten-horizon
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

if [[ "${MAMUT_TRACE_MIDI:-0}" != "0" ]]; then
  trace_midi=1
fi

while (($#)); do
  case "$1" in
    --audio-device)
      [[ $# -ge 2 ]] || {
        echo "error: --audio-device requires a value" >&2
        exit 1
      }
      audio_device="$2"
      shift 2
      ;;
    --alsa-period-frames)
      [[ $# -ge 2 ]] || {
        echo "error: --alsa-period-frames requires a value" >&2
        exit 1
      }
      alsa_period_frames="$2"
      shift 2
      ;;
    --alsa-buffer-frames)
      [[ $# -ge 2 ]] || {
        echo "error: --alsa-buffer-frames requires a value" >&2
        exit 1
      }
      alsa_buffer_frames="$2"
      shift 2
      ;;
    --alsa-start-threshold-frames)
      [[ $# -ge 2 ]] || {
        echo "error: --alsa-start-threshold-frames requires a value" >&2
        exit 1
      }
      alsa_start_threshold_frames="$2"
      shift 2
      ;;
    --midi-device)
      [[ $# -ge 2 ]] || {
        echo "error: --midi-device requires a value" >&2
        exit 1
      }
      midi_device="$2"
      shift 2
      ;;
    --midi-channel)
      [[ $# -ge 2 ]] || {
        echo "error: --midi-channel requires a value" >&2
        exit 1
      }
      midi_channel="$2"
      shift 2
      ;;
    --controller-profile)
      [[ $# -ge 2 ]] || {
        echo "error: --controller-profile requires a value" >&2
        exit 1
      }
      controller_profile="$2"
      shift 2
      ;;
    --trace-midi)
      trace_midi=1
      shift
      ;;
    --demo)
      demo=1
      shift
      ;;
    --windowed)
      headless=0
      shift
      ;;
    --headless)
      headless=1
      shift
      ;;
    --debug)
      cargo_profile="debug"
      shift
      ;;
    --release)
      cargo_profile="release"
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    -*)
      echo "error: unknown option $1" >&2
      usage >&2
      exit 1
      ;;
    *)
      patch_name="$1"
      shift
      if (($#)); then
        echo "error: unexpected extra arguments: $*" >&2
        usage >&2
        exit 1
      fi
      ;;
  esac
done

if [[ -z "$audio_device" ]]; then
  audio_device="$(detect_ag_audio_device || true)"
fi

if [[ -z "$audio_device" ]]; then
  echo "error: no AG06/AG03 ALSA hw playback device detected; run \`cargo run --locked -p mamut-standalone -- list-audio\` and pass --audio-device hw:<card>,<device>" >&2
  exit 1
fi

case "$cargo_profile" in
  debug|release)
    ;;
  *)
    echo "error: MAMUT_CARGO_PROFILE must be debug or release, got: $cargo_profile" >&2
    exit 1
    ;;
esac

mode_label="headless"
cmd=(cargo run)

if [[ "$cargo_profile" == "release" ]]; then
  cmd+=(--release)
fi

cmd+=(
  --locked
  --manifest-path "$repo_root/Cargo.toml" \
  -p mamut-standalone \
  -- \
  play
  --audio-device "$audio_device"
)

if [[ -n "$alsa_period_frames" ]]; then
  cmd+=(--alsa-period-frames "$alsa_period_frames")
fi

if [[ -n "$alsa_buffer_frames" ]]; then
  cmd+=(--alsa-buffer-frames "$alsa_buffer_frames")
fi

if [[ -n "$alsa_start_threshold_frames" ]]; then
  cmd+=(--alsa-start-threshold-frames "$alsa_start_threshold_frames")
fi

if ((headless)); then
  cmd+=(--headless)
else
  mode_label="windowed"
fi

if ((demo)); then
  cmd+=(--demo)
fi

if ((trace_midi)); then
  cmd+=(--trace-midi)
fi

cmd+=(
  --midi-device "$midi_device"
  --midi-channel "$midi_channel"
)

if [[ -n "$controller_profile" ]]; then
  cmd+=(--controller-profile "$controller_profile")
fi

export MAMUT_CAPTURE_DIR="$capture_dir"

cmd+=("$patch_name")

printf 'Launching EPM1 standalone\n'
printf '  patch: %s\n' "$patch_name"
printf '  audio: %s\n' "$audio_device"
if [[ -n "$alsa_period_frames" || -n "$alsa_buffer_frames" || -n "$alsa_start_threshold_frames" ]]; then
  printf '  alsa tuning: period=%s buffer=%s start_threshold=%s\n' \
    "${alsa_period_frames:-default}" \
    "${alsa_buffer_frames:-default}" \
    "${alsa_start_threshold_frames:-default}"
fi
printf '  midi: %s\n' "$midi_device"
printf '  midi channel: %s\n' "$midi_channel"
printf '  controller profile: %s\n' "${controller_profile:-none}"
printf '  capture dir: %s\n' "$capture_dir"
printf '  cargo profile: %s\n' "$cargo_profile"
if ((trace_midi)); then
  printf '  midi trace: enabled\n'
else
  printf '  midi trace: disabled\n'
fi
printf '  mode: %s\n' "$mode_label"

exec "${cmd[@]}"
