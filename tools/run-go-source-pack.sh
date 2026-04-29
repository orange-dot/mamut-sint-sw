#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

sources_dir="${GO_SOURCE_PACK_SOURCES:-$repo_root/.pdf-extract/sources/go}"
runs_dir="${GO_SOURCE_PACK_RUNS:-$repo_root/.pdf-extract/runs}"
chunk_chars="${GO_SOURCE_PACK_CHUNK_CHARS:-6000}"
dry_run=0

usage() {
  cat <<EOF
Usage:
  tools/run-go-source-pack.sh [--dry-run]

Purpose:
  Populate the local Go source-pack runs used by the idiomatic-go-writer skill.

Expected staged sources in:
  $sources_dir

Canonical source file names:
  - the-go-programming-language.pdf
  - the-go-programming-language.epub
  - effective-go.html
  - effective-go.md
  - 100-go-mistakes.pdf
  - 100-go-mistakes.epub

Canonical run outputs:
  - $runs_dir/go-programming-language-fitz   (PDF path)
  - $runs_dir/go-programming-language-source (EPUB fallback)
  - $runs_dir/effective-go-source
  - $runs_dir/100-go-mistakes-fitz           (PDF path)
  - $runs_dir/100-go-mistakes-source         (EPUB fallback)

Canonical symlink-style aliases refreshed by this script:
  - $runs_dir/go-programming-language
  - $runs_dir/effective-go
  - $runs_dir/100-go-mistakes
EOF
}

note() {
  printf '==> %s\n' "$*"
}

die() {
  printf 'error: %s\n' "$*" >&2
  exit 1
}

run_cmd() {
  if [ "$dry_run" -eq 1 ]; then
    printf '+'
    printf ' %q' "$@"
    printf '\n'
    return 0
  fi
  "$@"
}

ensure_dir() {
  if [ "$dry_run" -eq 1 ]; then
    printf '+ mkdir -p %q\n' "$1"
    return 0
  fi
  mkdir -p "$1"
}

pick_first_existing() {
  for candidate in "$@"; do
    if [ -f "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

refresh_alias() {
  local target="$1"
  local alias_path="$2"
  if [ "$dry_run" -eq 1 ]; then
    printf '+ ln -sfn %q %q\n' "$(basename "$target")" "$alias_path"
    return 0
  fi
  ln -sfn "$(basename "$target")" "$alias_path"
}

while [ "$#" -gt 0 ]; do
  case "$1" in
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

ensure_dir "$runs_dir"

primary_source="$(pick_first_existing \
  "$sources_dir/the-go-programming-language.pdf" \
  "$sources_dir/the-go-programming-language.epub")" || die "missing staged source for The Go Programming Language"

effective_source="$(pick_first_existing \
  "$sources_dir/effective-go.md" \
  "$sources_dir/effective-go.html")" || die "missing staged source for Effective Go"

mistakes_source="$(pick_first_existing \
  "$sources_dir/100-go-mistakes.pdf" \
  "$sources_dir/100-go-mistakes.epub")" || die "missing staged source for 100 Go Mistakes"

case "${primary_source##*.}" in
  pdf|PDF)
    primary_run="$runs_dir/go-programming-language-fitz"
    note "extracting The Go Programming Language via PDF extractor"
    run_cmd "$script_dir/extract-pdf-md.sh" "$primary_source" \
      --output "$primary_run" \
      --engine fitz \
      --chunk-chars "$chunk_chars" \
      --overwrite
    ;;
  epub|EPUB)
    primary_run="$runs_dir/go-programming-language-source"
    note "extracting The Go Programming Language via source-doc extractor"
    run_cmd "$script_dir/extract-source-doc-md.sh" "$primary_source" \
      --output "$primary_run" \
      --title "The Go Programming Language" \
      --chunk-chars "$chunk_chars" \
      --overwrite
    ;;
  *)
    die "unsupported staged source for The Go Programming Language: $primary_source"
    ;;
esac

effective_run="$runs_dir/effective-go-source"
note "extracting Effective Go via source-doc extractor"
run_cmd "$script_dir/extract-source-doc-md.sh" "$effective_source" \
  --output "$effective_run" \
  --title "Effective Go" \
  --chunk-chars "$chunk_chars" \
  --overwrite

case "${mistakes_source##*.}" in
  pdf|PDF)
    mistakes_run="$runs_dir/100-go-mistakes-fitz"
    note "extracting 100 Go Mistakes via PDF extractor"
    run_cmd "$script_dir/extract-pdf-md.sh" "$mistakes_source" \
      --output "$mistakes_run" \
      --engine fitz \
      --chunk-chars "$chunk_chars" \
      --overwrite
    ;;
  epub|EPUB)
    mistakes_run="$runs_dir/100-go-mistakes-source"
    note "extracting 100 Go Mistakes via source-doc extractor"
    run_cmd "$script_dir/extract-source-doc-md.sh" "$mistakes_source" \
      --output "$mistakes_run" \
      --title "100 Go Mistakes and How to Avoid Them" \
      --chunk-chars "$chunk_chars" \
      --overwrite
    ;;
  *)
    die "unsupported staged source for 100 Go Mistakes: $mistakes_source"
    ;;
esac

refresh_alias "$primary_run" "$runs_dir/go-programming-language"
refresh_alias "$effective_run" "$runs_dir/effective-go"
refresh_alias "$mistakes_run" "$runs_dir/100-go-mistakes"

note "Go source-pack runs are ready under $runs_dir"
