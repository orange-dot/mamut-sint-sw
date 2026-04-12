#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/../.." && pwd)"
agent_path="$repo_root/.claude/agents/sel4-rust-single-file-reviewer.md"

usage() {
  cat <<'EOF'
Usage:
  tools/review/run-rust-file-review.sh <repo-relative-rust-file>

Example:
  tools/review/run-rust-file-review.sh crates/mamut-engine/src/lib.rs
EOF
}

if [ "$#" -ne 1 ]; then
  usage >&2
  exit 2
fi

target="$1"
target_abs="$repo_root/$target"

if [ ! -f "$target_abs" ]; then
  echo "error: file not found: $target" >&2
  exit 2
fi

case "$target" in
  *.rs) ;;
  *)
    echo "error: target must be a Rust source file" >&2
    exit 2
    ;;
esac

echo "AGENT: $agent_path"
echo "SCOPE: $target"
echo
echo "RECOMMENDED PROMPT"
cat <<EOF
Review exactly this file with \`sel4-rust-single-file-reviewer\`.
Focus on file-local correctness, ownership discipline, panic paths, trait/type honesty, and hot-path contamination.
Target file: $target
EOF
echo
echo "RECENT DIFF CONTEXT"
git -C "$repo_root" diff --stat HEAD -- "$target" || true
