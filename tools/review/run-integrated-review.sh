#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
agent_path="$repo_root/.claude/agents/sel4-integrated-systems-reviewer.md"

usage() {
  cat <<'EOF'
Usage:
  tools/review/run-integrated-review.sh <path> [path...]

Examples:
  tools/review/run-integrated-review.sh crates/mamut-engine README.md docs
  tools/review/run-integrated-review.sh crates/mamut-patch docs/review-gates.md

If no paths are passed, the script falls back to changed implementation/docs files
from the working tree, or the last commit if the worktree is clean.
EOF
}

collect_default_scope() {
  if ! git -C "$repo_root" diff --quiet -- .; then
    git -C "$repo_root" diff --name-only HEAD -- README.md docs .claude crates tools
  elif git -C "$repo_root" rev-parse --verify HEAD~1 >/dev/null 2>&1; then
    git -C "$repo_root" diff --name-only HEAD~1..HEAD -- README.md docs .claude crates tools
  fi
}

if [ "$#" -eq 0 ]; then
  mapfile -t scope < <(collect_default_scope)
else
  scope=("$@")
fi

if [ "${#scope[@]}" -eq 0 ]; then
  usage >&2
  echo "error: no integrated review scope resolved" >&2
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
Run an integrated systems review with \`sel4-integrated-systems-reviewer\`.
Judge code, docs, contracts, and subsystem boundaries together.
Integrated scope:
$(printf '  - %s\n' "${scope[@]}")
EOF
echo
echo "CHANGED FILES"
git -C "$repo_root" diff --name-only HEAD -- "${scope[@]}" || true
