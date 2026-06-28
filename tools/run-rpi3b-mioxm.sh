#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." 2>/dev/null && pwd || true)"

if [[ -n "${MAMUT_BIN:-}" ]]; then
  mamut_bin="$MAMUT_BIN"
  app_root="$(cd "$(dirname "$mamut_bin")/.." && pwd)"
elif [[ -x "$script_dir/bin/mamut-standalone" ]]; then
  app_root="$script_dir"
  mamut_bin="$app_root/bin/mamut-standalone"
elif [[ -n "$repo_root" && -x "$repo_root/target/aarch64-unknown-linux-gnu/release/mamut-standalone" ]]; then
  app_root="$repo_root"
  mamut_bin="$repo_root/target/aarch64-unknown-linux-gnu/release/mamut-standalone"
else
  app_root="${repo_root:-$script_dir}"
  mamut_bin="mamut-standalone"
fi

patch_name="${MAMUT_PATCH:-molten-horizon}"
audio_device="${MAMUT_RPI_AUDIO_DEVICE:-${MAMUT_AUDIO_DEVICE:-}}"
sample_rate="${MAMUT_SAMPLE_RATE:-48000}"
alsa_period_frames="${MAMUT_ALSA_PERIOD_FRAMES:-512}"
alsa_buffer_frames="${MAMUT_ALSA_BUFFER_FRAMES:-2048}"
alsa_start_threshold_frames="${MAMUT_ALSA_START_THRESHOLD_FRAMES:-2048}"
midi_device="${MAMUT_MIDI_DEVICE:-mioXM DIN 1}"
midi_channel="${MAMUT_MIDI_CHANNEL:-2}"
controller_profile="${MAMUT_CONTROLLER_PROFILE:-$app_root/profiles/pc4-full.toml}"
capture_dir="${MAMUT_CAPTURE_DIR:-$app_root/captures}"
trace_midi=0
demo=0
tui=0
list_only=0

usage() {
  cat <<'EOF'
usage: run-rpi3b-mioxm.sh [options] [factory-name-or-path]

Run EPM1 on Raspberry Pi OS Lite with PC4 -> mioXM USB MIDI and Pi analog audio.

options:
  --audio-device <hw:card,device> Override detected Pi analog ALSA output
  --sample-rate <hz>              Override sample rate (default: 48000)
  --alsa-period-frames <n>        ALSA period frames (default: 512)
  --alsa-buffer-frames <n>        ALSA buffer frames (default: 2048)
  --alsa-start-threshold-frames <n>
                                  ALSA start threshold (default: 2048)
  --midi-device <name-or-index>   MIDI input selector (default: mioXM DIN 1)
  --midi-channel <1..16>          MIDI channel filter (default: 2)
  --controller-profile <path>     Controller TOML profile
  --trace-midi                    Log incoming MIDI messages to stderr
  --demo                          Use built-in demo performer
  --tui                           No GUI, but keep terminal controls over SSH
  --list                          Run list-audio and list-midi preflight only
  -h, --help                      Show this help

environment overrides:
  MAMUT_BIN
  MAMUT_PATCH
  MAMUT_RPI_AUDIO_DEVICE
  MAMUT_SAMPLE_RATE
  MAMUT_MIDI_DEVICE
  MAMUT_MIDI_CHANNEL
  MAMUT_CONTROLLER_PROFILE
  MAMUT_CAPTURE_DIR
  MAMUT_TRACE_MIDI
  MAMUT_RPI_ANALOG_CARD_PATTERN
EOF
}

detect_pi_analog_audio_device() {
  local card_pattern
  local card_index

  card_pattern="${MAMUT_RPI_ANALOG_CARD_PATTERN:-Headphones|bcm2835 Headphones|bcm2835 ALSA}"
  [[ -r /proc/asound/cards && -r /proc/asound/pcm ]] || return 1

  card_index="$(awk -v pattern="$card_pattern" '
    BEGIN { IGNORECASE = 1 }
    $0 ~ pattern {
      print $1
      exit
    }
  ' /proc/asound/cards)"
  [[ -n "${card_index:-}" ]] || return 1

  awk -v card="$card_index" '
    {
      prefix = $1
      sub(":", "", prefix)
      split(prefix, parts, "-")
      if ((parts[1] + 0) == (card + 0) && $0 ~ /playback/) {
        printf "hw:%d,%d\n", parts[1], parts[2]
        found = 1
        exit
      }
    }
    END { if (!found) exit 1 }
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
    --sample-rate)
      [[ $# -ge 2 ]] || {
        echo "error: --sample-rate requires a value" >&2
        exit 1
      }
      sample_rate="$2"
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
    --tui)
      tui=1
      shift
      ;;
    --list)
      list_only=1
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

if [[ "$list_only" == "1" ]]; then
  "$mamut_bin" list-audio
  "$mamut_bin" list-midi
  exit 0
fi

if [[ -z "$audio_device" ]]; then
  audio_device="$(detect_pi_analog_audio_device || true)"
fi

if [[ -z "$audio_device" ]]; then
  echo "error: no Pi analog ALSA hw playback device detected" >&2
  echo "hint: run '$mamut_bin list-audio' and pass --audio-device hw:<card>,<device>" >&2
  exit 1
fi

cmd=(
  "$mamut_bin"
  play
  --audio-device "$audio_device"
  --sample-rate "$sample_rate"
  --alsa-period-frames "$alsa_period_frames"
  --alsa-buffer-frames "$alsa_buffer_frames"
  --alsa-start-threshold-frames "$alsa_start_threshold_frames"
)

if [[ "$tui" == "0" ]]; then
  cmd+=(--headless)
fi

if [[ "$demo" == "1" ]]; then
  cmd+=(--demo)
fi

if [[ "$trace_midi" == "1" ]]; then
  cmd+=(--trace-midi)
fi

cmd+=(
  --midi-device "$midi_device"
  --midi-channel "$midi_channel"
)

if [[ -n "$controller_profile" ]]; then
  cmd+=(--controller-profile "$controller_profile")
fi

mkdir -p "$capture_dir"
export MAMUT_CAPTURE_DIR="$capture_dir"
cmd+=("$patch_name")

printf 'Launching EPM1 on Raspberry Pi\n'
printf '  binary: %s\n' "$mamut_bin"
printf '  patch: %s\n' "$patch_name"
printf '  audio: %s @ %s Hz\n' "$audio_device" "$sample_rate"
printf '  alsa tuning: period=%s buffer=%s start_threshold=%s\n' \
  "$alsa_period_frames" "$alsa_buffer_frames" "$alsa_start_threshold_frames"
printf '  midi: %s\n' "$midi_device"
printf '  midi channel: %s\n' "$midi_channel"
printf '  controller profile: %s\n' "${controller_profile:-none}"
printf '  capture dir: %s\n' "$capture_dir"
if [[ "$trace_midi" == "1" ]]; then
  printf '  midi trace: enabled\n'
else
  printf '  midi trace: disabled\n'
fi
if [[ "$tui" == "1" ]]; then
  printf '  mode: terminal TUI over SSH\n'
else
  printf '  mode: headless block-forever\n'
fi

exec "${cmd[@]}"
