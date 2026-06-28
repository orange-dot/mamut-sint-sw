#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
target="${MAMUT_RPI_TARGET:-aarch64-unknown-linux-gnu}"
cargo_profile="${MAMUT_CARGO_PROFILE:-release}"
cargo_bin="${CARGO:-cargo}"

usage() {
  cat <<'EOF'
usage: tools/build-rpi3b-headless.sh [options]

Cross-build the minimal EPM1 standalone binary for Raspberry Pi OS Lite 64-bit.

options:
  --target <triple>      Rust target triple (default: aarch64-unknown-linux-gnu)
  --debug               Build debug profile
  --release             Build release profile (default)
  -h, --help            Show this help

environment overrides:
  MAMUT_RPI_TARGET      Rust target triple
  MAMUT_CARGO_PROFILE   debug or release
  CARGO                 Cargo binary

expected host prerequisites for the default target:
  rustup target add aarch64-unknown-linux-gnu
  aarch64-linux-gnu-gcc
  arm64 libasound development files visible to pkg-config/linker
EOF
}

while (($#)); do
  case "$1" in
    --target)
      [[ $# -ge 2 ]] || {
        echo "error: --target requires a value" >&2
        exit 1
      }
      target="$2"
      shift 2
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
    *)
      echo "error: unknown option $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

case "$cargo_profile" in
  debug|release)
    ;;
  *)
    echo "error: MAMUT_CARGO_PROFILE must be debug or release, got: $cargo_profile" >&2
    exit 1
    ;;
esac

if [[ "$cargo_bin" == "cargo" && -x "$HOME/.cargo/bin/cargo" ]]; then
  cargo_bin="$HOME/.cargo/bin/cargo"
fi

if command -v rustup >/dev/null 2>&1 &&
  ! rustup target list --installed | grep -qx "$target"; then
  echo "error: Rust target '$target' is not installed" >&2
  echo "hint: rustup target add $target" >&2
  exit 1
fi

if [[ "$target" == "aarch64-unknown-linux-gnu" ]]; then
  export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER="${CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER:-aarch64-linux-gnu-gcc}"
  export PKG_CONFIG_ALLOW_CROSS="${PKG_CONFIG_ALLOW_CROSS:-1}"
  if ! command -v "$CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER" >/dev/null 2>&1; then
    echo "error: linker '$CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER' was not found" >&2
    echo "hint: install an aarch64 Linux cross toolchain or set CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER" >&2
    exit 1
  fi
fi

cmd=(
  "$cargo_bin" build
  --locked
  --manifest-path "$repo_root/Cargo.toml"
  -p mamut-standalone
  --target "$target"
  --no-default-features
)

if [[ "$cargo_profile" == "release" ]]; then
  cmd+=(--release)
fi

printf 'Building EPM1 RPi headless binary\n'
printf '  target: %s\n' "$target"
printf '  cargo profile: %s\n' "$cargo_profile"
printf '  gui feature: disabled\n'

"${cmd[@]}"

artifact_profile="$cargo_profile"
printf 'Built artifact:\n  %s\n' "$repo_root/target/$target/$artifact_profile/mamut-standalone"
