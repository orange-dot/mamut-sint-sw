#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

install_root="${PDF_EXTRACT_HOME:-$repo_root/.pdf-extract}"
profile="${PDF_EXTRACT_PROFILE:-max}"
torch_profile="${PDF_EXTRACT_TORCH_PROFILE:-auto}"
primary_python="${PDF_EXTRACT_PYTHON:-}"
marker_python="${PDF_EXTRACT_MARKER_PYTHON:-}"
mineru_python="${PDF_EXTRACT_MINERU_PYTHON:-}"
marker_mode="auto"
with_marker=1
with_mineru=0
dry_run=0

usage() {
  cat <<'EOF'
Usage:
  tools/install-pdf-extract-stack.sh [options]

Purpose:
  Bootstrap a repo-local PDF -> Markdown extraction stack aimed at LLM/RAG work.
  The script keeps the stable stack and the heavier stacks isolated in separate
  virtual environments under .pdf-extract/.

Default layout:
  .pdf-extract/
    venvs/docling/
    venvs/marker/
    venvs/mineru/

Default behavior:
  - installs the "max" profile
  - creates a Docling environment with VLM + OCR helpers
  - creates a Marker environment for higher-fidelity Markdown extraction when
    the local Python is compatible
  - skips MinerU unless explicitly requested

Options:
  --install-root PATH    Override the install root. Default: ./.pdf-extract
  --profile NAME         One of: stable, max. Default: max
  --python PATH          Preferred Python interpreter for docling / default envs
  --marker-python PATH   Python interpreter specifically for Marker
  --mineru-python PATH   Python interpreter specifically for MinerU
  --with-marker          Force-install Marker even in stable profile
  --without-marker       Skip Marker
  --with-mineru          Install MinerU if a compatible Python is available
  --torch PROFILE        One of: auto, cpu, default. Default: auto
  --dry-run              Print the work that would be done without changing anything
  --help                 Show this help

Environment overrides:
  PDF_EXTRACT_HOME
  PDF_EXTRACT_PROFILE
  PDF_EXTRACT_TORCH_PROFILE
  PDF_EXTRACT_PYTHON
  PDF_EXTRACT_MARKER_PYTHON
  PDF_EXTRACT_MINERU_PYTHON

Notes:
  - This script installs Python packages only. It does not apt/dnf/brew install
    system packages such as tesseract or poppler.
  - On Linux CPU-only machines, the default auto mode will use the PyTorch CPU
    wheel index recommended by Docling.
  - Marker is auto-skipped on Python 3.14+ unless you explicitly pass
    --with-marker together with a compatible --marker-python interpreter.
  - MinerU currently requires Python >=3.10 and <3.14, so it is optional here.
EOF
}

note() {
  printf '==> %s\n' "$*"
}

warn() {
  printf 'warning: %s\n' "$*" >&2
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

command_exists() {
  command -v "$1" >/dev/null 2>&1
}

resolve_python() {
  local explicit="${1:-}"
  local candidate

  if [ -n "$explicit" ]; then
    if [ -x "$explicit" ] || command_exists "$explicit"; then
      printf '%s\n' "$explicit"
      return 0
    fi
    die "python interpreter not found: $explicit"
  fi

  for candidate in python3.12 python3.11 python3.10 python3; do
    if command_exists "$candidate"; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done

  die "no supported python interpreter found (tried python3.12, python3.11, python3.10, python3)"
}

python_version() {
  local py="$1"
  "$py" -c 'import sys; print(".".join(str(part) for part in sys.version_info[:3]))'
}

python_satisfies() {
  local py="$1"
  local expr="$2"
  "$py" - "$expr" <<'PY'
import sys

expr = sys.argv[1]
major, minor = sys.version_info[:2]
ok = False

if expr == ">=3.10":
    ok = (major, minor) >= (3, 10)
elif expr == ">=3.10,<3.14":
    ok = (major, minor) >= (3, 10) and (major, minor) < (3, 14)
else:
    raise SystemExit(f"unsupported version expression: {expr}")

raise SystemExit(0 if ok else 1)
PY
}

needs_cpu_torch_index() {
  case "$torch_profile" in
    cpu)
      return 0
      ;;
    default)
      return 1
      ;;
    auto)
      if [ "$(uname -s)" = "Linux" ]; then
        if command_exists nvidia-smi && nvidia-smi >/dev/null 2>&1; then
          return 1
        fi
        return 0
      fi
      return 1
      ;;
    *)
      die "unsupported --torch profile: $torch_profile"
      ;;
  esac
}

pip_install() {
  local venv_python="$1"
  shift

  local -a cmd
  cmd=("$venv_python" -m pip install --upgrade)
  if needs_cpu_torch_index; then
    cmd+=(--extra-index-url "https://download.pytorch.org/whl/cpu")
  fi
  cmd+=("$@")

  if [ "$dry_run" -eq 1 ]; then
    printf '+'
    printf ' %q' "${cmd[@]}"
    printf '\n'
    return 0
  fi

  "${cmd[@]}"
}

ensure_venv() {
  local py="$1"
  local venv_dir="$2"

  if [ -x "$venv_dir/bin/python" ]; then
    note "reusing existing virtualenv $venv_dir"
    return 0
  fi

  note "creating virtualenv $venv_dir"
  if [ "$dry_run" -eq 1 ]; then
    printf '+ %q -m venv %q\n' "$py" "$venv_dir"
    return 0
  fi

  "$py" -m venv "$venv_dir"
}

write_activate_helper() {
  local target="$1"
  local venv_dir="$2"

  if [ "$dry_run" -eq 1 ]; then
    printf '+ write helper %s -> %s\n' "$target" "$venv_dir"
    return 0
  fi

  cat >"$target" <<EOF
#!/usr/bin/env bash
set -euo pipefail
source "$venv_dir/bin/activate"
exec "\${SHELL:-/bin/bash}" -i
EOF
  chmod +x "$target"
}

bootstrap_base_python_tools() {
  local venv_python="$1"
  pip_install "$venv_python" pip setuptools wheel
}

install_docling_stack() {
  local py="$1"
  local venv_dir="$install_root/venvs/docling"
  local helper="$install_root/bin/activate-docling"

  local -a packages=(
    "docling[vlm,rapidocr]>=2.78.0,<3"
    "pymupdf4llm[ocr,layout]>=1.27.2.1,<2"
    "python-frontmatter>=1.1,<2"
    "PyYAML>=6,<7"
    "orjson>=3.10,<4"
  )

  note "docling stack: python $(python_version "$py") via $py"
  ensure_venv "$py" "$venv_dir"
  bootstrap_base_python_tools "$venv_dir/bin/python"
  pip_install "$venv_dir/bin/python" "${packages[@]}"
  write_activate_helper "$helper" "$venv_dir"
}

install_marker_stack() {
  local py="$1"
  local venv_dir="$install_root/venvs/marker"
  local helper="$install_root/bin/activate-marker"

  local -a packages=(
    "marker-pdf>=1.10.2,<2"
    "python-frontmatter>=1.1,<2"
    "PyYAML>=6,<7"
    "orjson>=3.10,<4"
  )

  note "marker stack: python $(python_version "$py") via $py"
  ensure_venv "$py" "$venv_dir"
  bootstrap_base_python_tools "$venv_dir/bin/python"
  pip_install "$venv_dir/bin/python" "${packages[@]}"
  write_activate_helper "$helper" "$venv_dir"
}

install_mineru_stack() {
  local py="$1"
  local venv_dir="$install_root/venvs/mineru"
  local helper="$install_root/bin/activate-mineru"

  local -a packages=(
    "mineru[pipeline]>=2.7.6,<3"
    "python-frontmatter>=1.1,<2"
    "PyYAML>=6,<7"
    "orjson>=3.10,<4"
  )

  note "mineru stack: python $(python_version "$py") via $py"
  ensure_venv "$py" "$venv_dir"
  bootstrap_base_python_tools "$venv_dir/bin/python"
  pip_install "$venv_dir/bin/python" "${packages[@]}"
  write_activate_helper "$helper" "$venv_dir"
}

verify_environment() {
  local label="$1"
  local venv_python="$2"
  shift 2

  local -a checks=("$@")

  note "verifying $label"
  if [ "$dry_run" -eq 1 ]; then
    local item
    for item in "${checks[@]}"; do
      printf '+ %q -c %q\n' "$venv_python" "$item"
    done
    return 0
  fi

  local item
  for item in "${checks[@]}"; do
    "$venv_python" -c "$item"
  done
}

while [ "$#" -gt 0 ]; do
  case "$1" in
    --install-root)
      [ "$#" -ge 2 ] || die "--install-root requires a value"
      install_root="$2"
      shift 2
      ;;
    --profile)
      [ "$#" -ge 2 ] || die "--profile requires a value"
      profile="$2"
      shift 2
      ;;
    --python)
      [ "$#" -ge 2 ] || die "--python requires a value"
      primary_python="$2"
      shift 2
      ;;
    --marker-python)
      [ "$#" -ge 2 ] || die "--marker-python requires a value"
      marker_python="$2"
      shift 2
      ;;
    --mineru-python)
      [ "$#" -ge 2 ] || die "--mineru-python requires a value"
      mineru_python="$2"
      shift 2
      ;;
    --with-marker)
      marker_mode="force"
      with_marker=1
      shift
      ;;
    --without-marker)
      marker_mode="off"
      with_marker=0
      shift
      ;;
    --with-mineru)
      with_mineru=1
      shift
      ;;
    --torch)
      [ "$#" -ge 2 ] || die "--torch requires a value"
      torch_profile="$2"
      shift 2
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    --help|-h)
      usage
      exit 0
      ;;
    *)
      die "unknown argument: $1"
      ;;
  esac
done

case "$profile" in
  stable)
    if [ "$marker_mode" = "auto" ]; then
      with_marker=0
    fi
    ;;
  max)
    ;;
  *)
    die "unsupported profile: $profile"
    ;;
esac

primary_python="$(resolve_python "$primary_python")"

if ! python_satisfies "$primary_python" ">=3.10"; then
  die "docling stack requires Python >=3.10, got $(python_version "$primary_python")"
fi

if [ "$with_marker" -eq 1 ]; then
  marker_python="$(resolve_python "${marker_python:-$primary_python}")"
  if ! python_satisfies "$marker_python" ">=3.10"; then
    die "marker stack requires Python >=3.10, got $(python_version "$marker_python")"
  fi

  if [ "$marker_mode" = "auto" ] && ! python_satisfies "$marker_python" ">=3.10,<3.14"; then
    warn "skipping Marker: Python $(python_version "$marker_python") on $marker_python is outside the known-good range (<3.14)."
    warn "use --with-marker --marker-python <python3.12> if you want to force a compatible Marker install."
    with_marker=0
  fi
fi

if [ "$with_mineru" -eq 1 ]; then
  mineru_python="$(resolve_python "${mineru_python:-}")"
  if ! python_satisfies "$mineru_python" ">=3.10,<3.14"; then
    die "mineru requires Python >=3.10 and <3.14, got $(python_version "$mineru_python") from $mineru_python"
  fi
fi

note "install root: $install_root"
note "profile: $profile"
note "torch profile: $torch_profile"
if [ "$with_marker" -eq 1 ]; then
  note "marker mode: $marker_mode"
else
  note "marker mode: skipped"
fi

if [ "$dry_run" -eq 0 ]; then
  mkdir -p "$install_root/venvs" "$install_root/bin"
fi

install_docling_stack "$primary_python"
verify_environment \
  "docling" \
  "$install_root/venvs/docling/bin/python" \
  "import docling; import pymupdf4llm; import frontmatter; print('docling ok')" \
  "import importlib.metadata as m; print('docling', m.version('docling')); print('pymupdf4llm', m.version('pymupdf4llm'))"

if [ "$with_marker" -eq 1 ]; then
  if install_marker_stack "$marker_python"; then
    verify_environment \
      "marker" \
      "$install_root/venvs/marker/bin/python" \
      "import marker; print('marker ok')" \
      "import importlib.metadata as m; print('marker-pdf', m.version('marker-pdf'))"
  elif [ "$marker_mode" = "auto" ]; then
    warn "Marker installation failed in auto mode; continuing with the working Docling stack."
    warn "retry with --with-marker --marker-python <compatible-python> after adding the needed system libraries if you still want Marker."
    with_marker=0
  else
    exit 1
  fi
fi

if [ "$with_mineru" -eq 1 ]; then
  install_mineru_stack "$mineru_python"
  verify_environment \
    "mineru" \
    "$install_root/venvs/mineru/bin/python" \
    "import mineru; print('mineru ok')" \
    "import importlib.metadata as m; print('mineru', m.version('mineru'))"
fi

cat <<EOF

Install plan complete.

Useful next steps:
  source "$install_root/venvs/docling/bin/activate"
  "$install_root/bin/activate-docling"

Optional helpers:
$(if [ "$with_marker" -eq 0 ] && [ "$with_mineru" -eq 0 ]; then printf '  (none)\n'; fi)
$(if [ "$with_marker" -eq 1 ]; then printf '  "%s/bin/activate-marker"\n' "$install_root"; fi)
$(if [ "$with_mineru" -eq 1 ]; then printf '  "%s/bin/activate-mineru"\n' "$install_root"; fi)

Suggested first runs after install:
  docling --help
  python -c "import pymupdf4llm; print(pymupdf4llm.to_markdown('/path/to/file.pdf')[:500])"
  tools/extract-pdf-md.sh /path/to/file.pdf
$(if [ "$with_marker" -eq 1 ]; then printf '  %s\n' "marker_single /path/to/file.pdf --output_format markdown"; fi)
$(if [ "$with_mineru" -eq 1 ]; then printf '  %s\n' "mineru -p /path/to/file.pdf -o /tmp/mineru-out -b pipeline"; fi)
EOF
