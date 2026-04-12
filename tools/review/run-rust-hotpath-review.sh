#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
agent_path="$repo_root/.claude/agents/sel4-rust-execution-optimizer.md"

usage() {
  cat <<'EOF'
Usage:
  tools/review/run-rust-hotpath-review.sh <path> [path...]

Examples:
  tools/review/run-rust-hotpath-review.sh crates/mamut-dsp/src/lib.rs crates/mamut-engine/src/lib.rs
  tools/review/run-rust-hotpath-review.sh crates/mamut-standalone/src/main.rs

If no paths are passed, the script falls back to changed hot-path candidates under
`mamut-dsp`, `mamut-engine`, and `mamut-standalone`.
EOF
}

collect_default_scope() {
  if ! git -C "$repo_root" diff --quiet -- .; then
    git -C "$repo_root" diff --name-only HEAD -- \
      crates/mamut-dsp \
      crates/mamut-engine \
      crates/mamut-standalone
  elif git -C "$repo_root" rev-parse --verify HEAD~1 >/dev/null 2>&1; then
    git -C "$repo_root" diff --name-only HEAD~1..HEAD -- \
      crates/mamut-dsp \
      crates/mamut-engine \
      crates/mamut-standalone
  fi
}

if [ "$#" -eq 0 ]; then
  mapfile -t scope < <(collect_default_scope)
else
  scope=("$@")
fi

if [ "${#scope[@]}" -eq 0 ]; then
  usage >&2
  echo "error: no hot-path scope resolved" >&2
  exit 2
fi

echo "AGENT: $agent_path"
echo "SCOPE:"
for item in "${scope[@]}"; do
  echo "- $item"
done
echo
echo "RECOMMENDED PROMPT"
cat <<EOF
Run a hot-path review with \`sel4-rust-execution-optimizer\`.
Focus on steady-state allocations, cloning, logging, callback contamination, branchiness, and cache-hostile structure.
Hot-path scope:
$(printf '  - %s\n' "${scope[@]}")
EOF
echo
echo "CHANGED FILES"
git -C "$repo_root" diff --name-only HEAD -- "${scope[@]}" || true
