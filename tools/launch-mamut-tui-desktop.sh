#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
log_dir="${XDG_CACHE_HOME:-$HOME/.cache}/mamut"
mkdir -p "$log_dir"

cd "$repo_root"

export PATH="$HOME/.cargo/bin:/usr/local/cargo/bin:$PATH"
export MAMUT_MIDI_CHANNEL="${MAMUT_MIDI_CHANNEL:-2}"

exec "$repo_root/tools/run-pc4-ag03.sh" \
  --release \
  --headless \
  --trace-midi \
  molten-horizon \
  >>"$log_dir/tui-desktop.log" 2>&1
