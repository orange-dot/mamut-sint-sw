#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

preferred_python="${PDF_EXTRACT_RUNTIME_PYTHON:-$repo_root/.pdf-extract/venvs/docling/bin/python}"

if [ -x "$preferred_python" ]; then
  python_bin="$preferred_python"
elif command -v python3 >/dev/null 2>&1; then
  python_bin="$(command -v python3)"
else
  echo "error: no usable python interpreter found" >&2
  exit 1
fi

exec "$python_bin" "$script_dir/extract_source_doc_markdown.py" "$@"
