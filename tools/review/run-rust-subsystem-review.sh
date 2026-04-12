#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
agent_path="$repo_root/.claude/agents/sel4-rust-systems-reviewer.md"

usage() {
  cat <<'EOF'
Usage:
  tools/review/run-rust-subsystem-review.sh <path> [path...]

Examples:
  tools/review/run-rust-subsystem-review.sh crates/mamut-engine
  tools/review/run-rust-subsystem-review.sh crates/mamut-dsp crates/mamut-engine

If no paths are passed, the script falls back to changed Rust/Cargo files from the
working tree, or the last commit if the worktree is clean.
EOF
}

collect_default_scope() {
  if ! git -C "$repo_root" diff --quiet -- .; then
    git -C "$repo_root" diff --name-only HEAD -- '*.rs' 'Cargo.toml' 'Cargo.lock'
  elif git -C "$repo_root" rev-parse --verify HEAD~1 >/dev/null 2>&1; then
    git -C "$repo_root" diff --name-only HEAD~1..HEAD -- '*.rs' 'Cargo.toml' 'Cargo.lock'
  fi
}

if [ "$#" -eq 0 ]; then
  mapfile -t scope < <(collect_default_scope)
else
  scope=("$@")
fi

if [ "${#scope[@]}" -eq 0 ]; then
  usage >&2
  echo "error: no subsystem scope resolved" >&2
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
Run a strict subsystem review with \`sel4-rust-systems-reviewer\`.
Use \`sel4-rust-single-file-reviewer\` first for the riskiest files, then compose the subsystem verdict.
Subsystem scope:
$(printf '  - %s\n' "${scope[@]}")
EOF
echo
echo "CHANGED FILES"
git -C "$repo_root" diff --name-only HEAD -- "${scope[@]}" || true
