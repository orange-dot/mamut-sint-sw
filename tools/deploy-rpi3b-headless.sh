#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="${MAMUT_RPI_TARGET:-aarch64-unknown-linux-gnu}"
deploy_dir="${MAMUT_PI_DEPLOY_DIR:-/home/pi/mamut-epm1}"
host="${MAMUT_PI_HOST:-}"
binary="${MAMUT_RPI_BINARY:-$repo_root/target/$target/release/mamut-standalone}"

usage() {
  cat <<'EOF'
usage: tools/deploy-rpi3b-headless.sh [options] <user@pi-host>

Deploy the RPi3B headless EPM1 bundle over SSH.

options:
  --host <user@host>        Raspberry Pi SSH target
  --target <triple>         Rust target triple used for the built binary
  --binary <path>           Built mamut-standalone binary
  --deploy-dir <path>       Remote deploy directory (default: /home/pi/mamut-epm1)
  -h, --help                Show this help

environment overrides:
  MAMUT_PI_HOST
  MAMUT_PI_DEPLOY_DIR
  MAMUT_RPI_TARGET
  MAMUT_RPI_BINARY
EOF
}

while (($#)); do
  case "$1" in
    --host)
      [[ $# -ge 2 ]] || {
        echo "error: --host requires a value" >&2
        exit 1
      }
      host="$2"
      shift 2
      ;;
    --target)
      [[ $# -ge 2 ]] || {
        echo "error: --target requires a value" >&2
        exit 1
      }
      target="$2"
      if [[ -z "${MAMUT_RPI_BINARY:-}" ]]; then
        binary="$repo_root/target/$target/release/mamut-standalone"
      fi
      shift 2
      ;;
    --binary)
      [[ $# -ge 2 ]] || {
        echo "error: --binary requires a value" >&2
        exit 1
      }
      binary="$2"
      shift 2
      ;;
    --deploy-dir)
      [[ $# -ge 2 ]] || {
        echo "error: --deploy-dir requires a value" >&2
        exit 1
      }
      deploy_dir="$2"
      shift 2
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
      if [[ -n "$host" ]]; then
        echo "error: host already set to '$host', unexpected extra argument '$1'" >&2
        exit 1
      fi
      host="$1"
      shift
      ;;
  esac
done

if [[ -z "$host" ]]; then
  echo "error: missing Raspberry Pi SSH target" >&2
  usage >&2
  exit 1
fi

if [[ "$host" =~ [[:space:]] ]]; then
  echo "error: host must not contain whitespace" >&2
  exit 1
fi

if [[ "$deploy_dir" =~ [[:space:]] ]]; then
  echo "error: deploy directory must not contain whitespace" >&2
  exit 1
fi

if [[ ! -x "$binary" ]]; then
  echo "error: binary not found or not executable: $binary" >&2
  echo "hint: run tools/build-rpi3b-headless.sh first, or pass --binary" >&2
  exit 1
fi

if ! command -v rsync >/dev/null 2>&1; then
  echo "error: rsync is required for deploy" >&2
  exit 1
fi

printf 'Deploying EPM1 RPi headless bundle\n'
printf '  host: %s\n' "$host"
printf '  deploy dir: %s\n' "$deploy_dir"
printf '  binary: %s\n' "$binary"

ssh "$host" "mkdir -p '$deploy_dir/bin' '$deploy_dir/captures'"
rsync -a "$binary" "$host:$deploy_dir/bin/mamut-standalone"
rsync -a "$repo_root/patches/" "$host:$deploy_dir/patches/"
rsync -a "$repo_root/profiles/" "$host:$deploy_dir/profiles/"
rsync -a "$repo_root/tools/run-rpi3b-mioxm.sh" "$host:$deploy_dir/run-rpi3b-mioxm.sh"
ssh "$host" "chmod +x '$deploy_dir/bin/mamut-standalone' '$deploy_dir/run-rpi3b-mioxm.sh'"

printf 'Remote launch command:\n  ssh %s %s/run-rpi3b-mioxm.sh\n' "$host" "$deploy_dir"
