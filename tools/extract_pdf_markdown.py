#!/usr/bin/env python3
from __future__ import annotations

import argparse
import contextlib
import json
import os
import re
import shutil
import sys
import unicodedata
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

try:
    import fitz  # PyMuPDF
except ImportError as exc:  # pragma: no cover
    raise SystemExit(
        "error: PyMuPDF is required. Install the docling stack first with "
        "`tools/install-pdf-extract-stack.sh`."
    ) from exc

try:
    import pymupdf4llm
except ImportError:
    pymupdf4llm = None

try:
    import yaml
except ImportError:
    yaml = None


DROP_PAGE_BOX_CLASSES = {"page-header", "page-footer"}
MONOSPACE_FONT_MARKERS = ("mono", "courier", "menlo", "consolas", "nimbusmon", "code")
CODE_PATTERN = re.compile(
    r"(#include\b|//|/\*|\*/|\bint\b|\bchar\b|\bdouble\b|\bsize_t\b|\bstruct\b|"
    r"\breturn\b|\bfor\s*\(|\bwhile\s*\(|\bif\s*\(|\bswitch\s*\(|"
    r"\b[a-zA-Z_]\w*\s*\(|\[[^\]]+\]\s*=|=\s*[^=]|\{|\}|;)"
)


@dataclass
class PageRecord:
    page_number: int
    text: str
    toc_items: list[list[Any]]
    metadata: dict[str, Any]


@dataclass
class TocRecord:
    index: int
    level: int
    title: str
    page_start: int
    page_end: int
    path: list[str]
    next_title: str | None
    next_page: int | None
    slug: str


@dataclass
class RenderedBlock:
    kind: str
    text: str
    language: str | None = None


def eprint(message: str) -> None:
    print(message, file=sys.stderr)


def slugify(value: str, *, max_length: int = 80) -> str:
    text = unicodedata.normalize("NFKD", value)
    text = text.encode("ascii", "ignore").decode("ascii")
    text = re.sub(r"[^A-Za-z0-9]+", "-", text)
    text = text.strip("-").lower()
    if not text:
        text = "section"
    return text[:max_length].strip("-") or "section"


def strip_markdown_noise(value: str) -> str:
    text = value
    text = re.sub(r"<!--.*?-->", " ", text)
    text = re.sub(r"`+", " ", text)
    text = re.sub(r"[*_#>\[\]\(\)]", " ", text)
    text = re.sub(r"\s+", " ", text)
    return text.strip(" .:-").casefold()


def title_variants(title: str) -> list[str]:
    variants = [title.strip(), title.strip().rstrip(".")]
    plain = re.sub(r"^\s*(?:[A-Za-z]?\d+(?:\.\d+)*\.?\s+)", "", title).strip()
    if plain:
        variants.extend([plain, plain.rstrip(".")])
    return [item for item in dict.fromkeys(item for item in variants if item)]


def find_title_offset(text: str, title: str) -> int | None:
    normalized_targets = [strip_markdown_noise(item) for item in title_variants(title)]
    normalized_targets = [item for item in normalized_targets if item]
    if not normalized_targets:
        return None

    offset = 0
    for line in text.splitlines(keepends=True):
        normalized_line = strip_markdown_noise(line)
        if normalized_line:
            for target in normalized_targets:
                if normalized_line == target or normalized_line.startswith(target) or target in normalized_line:
                    return offset
        offset += len(line)

    folded = text.casefold()
    for candidate in title_variants(title):
        idx = folded.find(candidate.casefold())
        if idx != -1:
            return idx
    return None


def merge_ranges(ranges: list[tuple[int, int]]) -> list[tuple[int, int]]:
    if not ranges:
        return []
    merged: list[list[int]] = []
    for start, end in sorted(ranges):
        if end <= start:
            continue
        if not merged or start > merged[-1][1]:
            merged.append([start, end])
        else:
            merged[-1][1] = max(merged[-1][1], end)
    return [(start, end) for start, end in merged]


def drop_box_classes(text: str, page_boxes: list[dict[str, Any]], classes: set[str]) -> str:
    ranges: list[tuple[int, int]] = []
    for box in page_boxes:
        if box.get("class") not in classes:
            continue
        pos = box.get("pos")
        if not isinstance(pos, (list, tuple)) or len(pos) != 2:
            continue
        start, end = int(pos[0]), int(pos[1])
        start = max(start, 0)
        end = min(end, len(text))
        ranges.append((start, end))

    merged = merge_ranges(ranges)
    if not merged:
        return text

    chunks: list[str] = []
    cursor = 0
    for start, end in merged:
        if cursor < start:
            chunks.append(text[cursor:start])
        cursor = end
    if cursor < len(text):
        chunks.append(text[cursor:])
    return "".join(chunks)


def cleanup_markdown(text: str) -> str:
    cleaned = text.replace("\x00", "")
    cleaned = cleaned.replace("\r\n", "\n").replace("\r", "\n")
    cleaned = re.sub(r"[ \t]+\n", "\n", cleaned)
    cleaned = re.sub(r"\n{3,}", "\n\n", cleaned)
    cleaned = cleaned.strip()
    return cleaned + "\n" if cleaned else ""


def is_structural_line(line: str) -> bool:
    stripped = line.strip()
    if not stripped:
        return True
    return (
        stripped.startswith(("```", "<!--", "#", "> "))
        or stripped.startswith("- ")
        or bool(re.match(r"^\[\d+\]\s", stripped))
    )


def cleanup_prose_lines(lines: list[str]) -> list[str]:
    cleaned: list[str] = []
    for raw_line in lines:
        line = raw_line.rstrip()
        if not line.strip():
            cleaned.append("")
            continue

        line = re.sub(r"^(\d+)([A-Z][a-z])", r"[\1] \2", line)
        line = re.sub(r"(?<=[.!?])\s+(\d+)(?=[A-Z])", r" [\1] ", line)
        line = re.sub(r"(?<=[A-Za-z”’'\")\]])\.(\d+)(?=\s|$)", r".[\1]", line)
        line = re.sub(r"\b([A-Za-z0-9_-]*[a-z][A-Za-z0-9_-]*)C\b", r"\1", line)
        line = re.sub(r"^Takeaway\s+(.+?#\d+)\s+(.+)$", r"> Takeaway \1: \2", line)
        line = re.sub(r"\s{2,}", " ", line).rstrip()
        cleaned.append(line if raw_line.startswith(("  ", "\t")) else line.strip())

    while cleaned and cleaned[-1] == "":
        cleaned.pop()
    return cleaned


def semantic_cleanup_markdown(text: str) -> str:
    if not text.strip():
        return ""

    parts = re.split(r"(```[\s\S]*?```)", text)
    cleaned_parts: list[str] = []
    for part in parts:
        if not part:
            continue
        if part.startswith("```") and part.endswith("```"):
            cleaned_parts.append(part.strip())
            continue
        lines = part.splitlines()
        cleaned_lines = cleanup_prose_lines(lines)
        prose = "\n".join(cleaned_lines).strip()
        if prose:
            cleaned_parts.append(prose)

    return cleanup_markdown("\n\n".join(cleaned_parts))


def yaml_frontmatter(data: dict[str, Any]) -> str:
    if yaml is not None:
        dumped = yaml.safe_dump(data, sort_keys=False, allow_unicode=True).strip()
        return f"---\n{dumped}\n---\n\n"

    def render_scalar(value: Any) -> str:
        if value is None:
            return "null"
        if isinstance(value, bool):
            return "true" if value else "false"
        if isinstance(value, (int, float)):
            return str(value)
        return json.dumps(str(value), ensure_ascii=False)

    lines = ["---"]
    for key, value in data.items():
        if isinstance(value, list):
            lines.append(f"{key}:")
            for item in value:
                lines.append(f"  - {render_scalar(item)}")
        else:
            lines.append(f"{key}: {render_scalar(value)}")
    lines.append("---")
    lines.append("")
    return "\n".join(lines) + "\n"


def write_markdown(path: Path, frontmatter: dict[str, Any], body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(yaml_frontmatter(frontmatter) + body.rstrip() + "\n", encoding="utf-8")


def write_json(path: Path, payload: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")


def drop_header_footer_block(text: str, y0: float, y1: float, page_height: float) -> bool:
    compact = " ".join(text.split())
    if not compact:
        return True
    if y1 <= 110:
        if re.fullmatch(r"[ivxlcdmIVXLCDM]+", compact):
            return True
        if re.fullmatch(r"\d+", compact):
            return True
        if re.fullmatch(r"\d+\s+[A-Z0-9 .:-]+", compact):
            return True
        if re.fullmatch(r"[A-Z0-9 .:-]+\s+\d+", compact):
            return True
        if compact.isupper() and len(compact) <= 90:
            return True
    if y0 >= page_height - 95 and len(compact) <= 12:
        if re.fullmatch(r"[ivxlcdmIVXLCDM]+", compact) or re.fullmatch(r"\d+", compact):
            return True
    return False


def is_monospace_font(font_name: str) -> bool:
    lowered = font_name.casefold()
    return any(marker in lowered for marker in MONOSPACE_FONT_MARKERS)


def iter_block_lines(block: dict[str, Any]) -> list[dict[str, Any]]:
    lines: list[dict[str, Any]] = []
    if "lines" in block:
        lines.extend(block["lines"])
    for child in block.get("blocks", []):
        lines.extend(iter_block_lines(child))
    return lines


def group_visual_rows(block: dict[str, Any]) -> list[list[dict[str, Any]]]:
    raw_lines = iter_block_lines(block)
    rows: list[list[dict[str, Any]]] = []
    current: list[dict[str, Any]] = []
    current_y: float | None = None

    for line in raw_lines:
        spans = line.get("spans") or []
        if not spans:
            continue
        y0 = float(line["bbox"][1])
        if current and current_y is not None and abs(y0 - current_y) <= 1.5:
            current.append(line)
            continue
        if current:
            rows.append(current)
        current = [line]
        current_y = y0

    if current:
        rows.append(current)

    return rows


def row_to_text(
    row: list[dict[str, Any]],
    *,
    drop_leading_number: bool = False,
    strip_bullet: bool = False,
) -> str:
    pieces: list[str] = []
    ordered = sorted(row, key=lambda item: item["bbox"][0])

    for subline in ordered:
        text = "".join(span["text"] for span in subline.get("spans", []))
        text = text.replace("\xa0", " ")
        if not text.strip():
            continue
        if drop_leading_number and not pieces and len(ordered) > 1 and text.strip().isdigit():
            continue
        if strip_bullet and not pieces:
            text = re.sub(r"^\s*[•·▪◦]\s*", "", text)
        if pieces and not pieces[-1].endswith((" ", "\t")) and not text.startswith((" ", "\t", ",", ".", ":", ";", ")", "]", "}")):
            pieces.append(" ")
        pieces.append(text)

    return "".join(pieces).rstrip()


def join_wrapped_lines(lines: list[str]) -> str:
    paragraphs: list[str] = []
    current = ""

    for line in lines:
        stripped = line.strip()
        if not stripped:
            if current:
                paragraphs.append(current)
                current = ""
            continue
        if stripped.startswith(("- ", "* ", "• ")):
            if current:
                paragraphs.append(current)
                current = ""
            paragraphs.append(stripped)
            continue
        if not current:
            current = stripped
            continue
        if current.endswith("-") and stripped[:1].islower():
            current = current[:-1] + stripped
        else:
            current = current + " " + stripped

    if current:
        paragraphs.append(current)

    return "\n".join(paragraphs)


def looks_like_code(text: str) -> bool:
    return bool(CODE_PATTERN.search(text))


def is_bullet_list_item(rows: list[list[dict[str, Any]]]) -> bool:
    for row in rows:
        text = row_to_text(row)
        if not text.strip():
            continue
        return bool(re.match(r"^\s*[•·▪◦]\s+", text))
    return False


def first_row_starts_with_number(rows: list[list[dict[str, Any]]]) -> bool:
    if not rows:
        return False
    ordered = sorted(rows[0], key=lambda item: item["bbox"][0])
    for subline in ordered:
        text = "".join(span["text"] for span in subline.get("spans", [])).strip()
        if not text:
            continue
        return text.isdigit()
    return False


def block_monospace_ratio(rows: list[list[dict[str, Any]]]) -> float:
    mono_chars = 0
    total_chars = 0
    for row in rows:
        for subline in row:
            for span in subline.get("spans", []):
                text = span.get("text", "")
                count = len(text)
                total_chars += count
                if is_monospace_font(span.get("font", "")):
                    mono_chars += count
    if total_chars == 0:
        return 0.0
    return mono_chars / total_chars


def should_merge_prose_blocks(left: str, right: str) -> bool:
    left = left.strip()
    right = right.strip()
    if not left or not right:
        return False
    right_first = right.splitlines()[0].strip()
    if left.startswith("- ") or right.startswith("- "):
        return False
    if left.startswith("> Takeaway ") or right.startswith("> Takeaway "):
        return False
    if left.startswith("```") or right.startswith("```"):
        return False
    if re.search(r"[.!?:]$", left):
        return False
    return bool(re.match(r"^[a-z0-9(<[]", right))


def merge_rendered_blocks(blocks: list[RenderedBlock]) -> list[RenderedBlock]:
    merged: list[RenderedBlock] = []
    for block in blocks:
        if block.kind == "fence" and block.text == "":
            if merged and merged[-1].kind == "fence" and merged[-1].language == block.language:
                merged[-1].text = merged[-1].text.rstrip() + "\n"
            continue
        text = block.text.strip()
        if not text:
            continue
        normalized = RenderedBlock(kind=block.kind, text=text, language=block.language)
        if merged:
            previous = merged[-1]
            if (
                previous.kind == "fence"
                and normalized.kind == "fence"
                and previous.language == normalized.language
            ):
                previous.text = previous.text.rstrip() + "\n" + normalized.text.lstrip()
                continue
            if previous.kind == "prose" and normalized.kind == "prose" and should_merge_prose_blocks(previous.text, normalized.text):
                previous.text = previous.text.rstrip() + " " + normalized.text.lstrip()
                continue
        merged.append(normalized)
    return merged


def render_fitz_blocks(page: fitz.Page, *, flags: int) -> str:
    rendered: list[RenderedBlock] = []
    terminal_mode = False
    code_mode = False
    page_height = page.rect.height

    prepared: list[dict[str, Any]] = []
    for block in page.get_text("dict", flags=flags)["blocks"]:
        rows = group_visual_rows(block)
        if not rows:
            continue
        compact_text = join_wrapped_lines([row_to_text(row) for row in rows])
        if drop_header_footer_block(compact_text, block["bbox"][1], block["bbox"][3], page_height):
            continue
        prepared.append(
            {
                "block": block,
                "rows": rows,
                "compact_text": compact_text,
                "mono_ratio": block_monospace_ratio(rows),
                "plain_lines": [row_to_text(row) for row in rows],
                "preformatted_lines": [row_to_text(row, drop_leading_number=True).rstrip() for row in rows],
                "is_bullet": block.get("type") == 2 and is_bullet_list_item(rows),
                "starts_with_number": first_row_starts_with_number(rows),
            }
        )

    for idx, info in enumerate(prepared):
        next_info = prepared[idx + 1] if idx + 1 < len(prepared) else None

        if info["is_bullet"]:
            list_text = join_wrapped_lines([row_to_text(row, strip_bullet=True) for row in info["rows"]]).strip()
            if list_text:
                rendered.append(RenderedBlock(kind="prose", text=f"- {list_text}"))
            terminal_mode = False
            code_mode = False
            continue

        prose_text = join_wrapped_lines(info["plain_lines"]).strip()
        if prose_text.casefold() == "terminal":
            rendered.append(RenderedBlock(kind="prose", text=prose_text))
            terminal_mode = True
            code_mode = False
            continue

        sanitized_lines = [
            "" if re.fullmatch(r"\d+", line.strip()) else line
            for line in info["preformatted_lines"]
        ]
        if sanitized_lines and sanitized_lines[-1] == "":
            if next_info and next_info["mono_ratio"] >= 0.75 and not next_info["starts_with_number"]:
                while sanitized_lines and sanitized_lines[-1] == "":
                    sanitized_lines.pop()

        preformatted_text = "\n".join(sanitized_lines).rstrip()
        mono_ratio = info["mono_ratio"]
        code_like = looks_like_code(preformatted_text)
        terminal_like = preformatted_text.lstrip().startswith((">", "$"))
        number_only = bool(info["compact_text"].strip()) and re.fullmatch(r"\d+", info["compact_text"].strip()) is not None
        code_continuation = code_mode and (number_only or info["starts_with_number"] or mono_ratio >= 0.45)

        if mono_ratio >= 0.75:
            if terminal_mode and (preformatted_text or number_only):
                rendered.append(RenderedBlock(kind="fence", text=preformatted_text, language="text"))
                code_mode = False
                continue

            if code_like or code_mode or len(info["rows"]) >= 2:
                if not preformatted_text and number_only:
                    keep_blank = next_info is None or next_info["starts_with_number"]
                    if keep_blank:
                        rendered.append(RenderedBlock(kind="fence", text="", language="c"))
                    continue
                rendered.append(RenderedBlock(kind="fence", text=preformatted_text, language="c"))
                code_mode = True
                terminal_mode = False
                continue

        if code_continuation:
            if not preformatted_text and number_only:
                keep_blank = next_info is None or next_info["starts_with_number"]
                if keep_blank:
                    rendered.append(RenderedBlock(kind="fence", text="", language="c"))
                continue
            rendered.append(RenderedBlock(kind="fence", text=preformatted_text, language="c"))
            terminal_mode = False
            code_mode = True
            continue

        code_mode = False
        terminal_mode = False
        if prose_text:
            rendered.append(RenderedBlock(kind="prose", text=prose_text))

    merged = merge_rendered_blocks(rendered)
    parts: list[str] = []
    for block in merged:
        if block.kind == "fence":
            language = block.language or ""
            parts.append(f"```{language}\n{block.text.rstrip()}\n```")
        else:
            parts.append(block.text.strip())
    return cleanup_markdown("\n\n".join(parts))


def extract_pages_with_fitz(doc: fitz.Document, max_pages: int | None) -> list[PageRecord]:
    limit = min(doc.page_count, max_pages or doc.page_count)
    flags = fitz.TEXT_DEHYPHENATE | fitz.TEXT_PRESERVE_WHITESPACE | fitz.TEXT_PARAGRAPH_BREAK
    pages: list[PageRecord] = []
    for idx in range(limit):
        page = doc.load_page(idx)
        pages.append(
            PageRecord(
                page_number=idx + 1,
                text=semantic_cleanup_markdown(render_fitz_blocks(page, flags=flags)),
                toc_items=[],
                metadata={"page_number": idx + 1},
            )
        )
    return pages


def extract_pages_with_pymupdf4llm(pdf_path: Path, max_pages: int | None) -> list[PageRecord]:
    if pymupdf4llm is None:
        raise RuntimeError("pymupdf4llm is not available")

    pages = list(range(max_pages)) if max_pages else None
    with contextlib.redirect_stdout(sys.stderr):
        chunks = pymupdf4llm.to_markdown(
            str(pdf_path),
            pages=pages,
            page_chunks=True,
            page_separators=False,
            force_text=True,
            ignore_code=False,
            extract_words=False,
            show_progress=False,
        )

    records: list[PageRecord] = []
    for idx, chunk in enumerate(chunks, start=1):
        metadata = dict(chunk.get("metadata") or {})
        page_number = int(metadata.get("page_number") or idx)
        text = str(chunk.get("text") or "")
        text = drop_box_classes(text, list(chunk.get("page_boxes") or []), DROP_PAGE_BOX_CLASSES)
        text = semantic_cleanup_markdown(cleanup_markdown(text))
        records.append(
            PageRecord(
                page_number=page_number,
                text=text,
                toc_items=list(chunk.get("toc_items") or []),
                metadata=metadata,
            )
        )
    return records


def build_toc_records(doc: fitz.Document, max_pages: int | None) -> list[TocRecord]:
    raw = doc.get_toc(simple=False)
    records: list[TocRecord] = []
    path_stack: list[str] = []
    limit = min(doc.page_count, max_pages or doc.page_count)

    for item in raw:
        level, title, page_start, _dest = item
        if page_start > limit:
            break
        path_stack = path_stack[: level - 1]
        path_stack.append(title)
        records.append(
            TocRecord(
                index=len(records) + 1,
                level=int(level),
                title=str(title),
                page_start=int(page_start),
                page_end=limit,
                path=list(path_stack),
                next_title=None,
                next_page=None,
                slug=f"{len(records) + 1:03d}-{slugify(title)}",
            )
        )

    for idx, record in enumerate(records):
        next_record = None
        for candidate in records[idx + 1 :]:
            if candidate.level <= record.level:
                next_record = candidate
                break
        if next_record is not None:
            record.next_title = next_record.title
            record.next_page = next_record.page_start
            record.page_end = min(next_record.page_start, limit)
        else:
            record.page_end = limit
    return records


def build_section_body(record: TocRecord, page_map: dict[int, PageRecord]) -> str:
    parts: list[str] = []
    last_page = record.page_end
    for page_number in range(record.page_start, last_page + 1):
        page = page_map.get(page_number)
        if page is None or not page.text.strip():
            continue
        text = page.text
        if page_number == record.page_start:
            start_idx = find_title_offset(text, record.title)
            if start_idx is not None:
                text = text[start_idx:]
        if record.next_title and record.next_page == page_number:
            end_idx = find_title_offset(text, record.next_title)
            if end_idx is not None and end_idx > 0:
                text = text[:end_idx]
        text = cleanup_markdown(text)
        if text:
            parts.append(f"<!-- page:{page_number:04d} -->\n\n{text.strip()}")
    return cleanup_markdown("\n\n".join(parts))


def split_large_paragraph(paragraph: str, max_chars: int) -> list[str]:
    if len(paragraph) <= max_chars:
        return [paragraph]

    lines = paragraph.splitlines()
    if len(lines) > 1:
        chunks: list[str] = []
        current: list[str] = []
        current_len = 0
        for line in lines:
            addition = len(line) + (1 if current else 0)
            if current and current_len + addition > max_chars:
                chunks.append("\n".join(current).strip())
                current = [line]
                current_len = len(line)
            else:
                current.append(line)
                current_len += addition
        if current:
            chunks.append("\n".join(current).strip())
        return [chunk for chunk in chunks if chunk]

    words = paragraph.split()
    chunks = []
    current_words: list[str] = []
    current_len = 0
    for word in words:
        addition = len(word) + (1 if current_words else 0)
        if current_words and current_len + addition > max_chars:
            chunks.append(" ".join(current_words))
            current_words = [word]
            current_len = len(word)
        else:
            current_words.append(word)
            current_len += addition
    if current_words:
        chunks.append(" ".join(current_words))
    return chunks or [paragraph]


def chunk_markdown(text: str, max_chars: int) -> list[str]:
    paragraphs = [item.strip() for item in re.split(r"\n{2,}", text.strip()) if item.strip()]
    if not paragraphs:
        return []

    chunks: list[str] = []
    current: list[str] = []
    current_len = 0

    for paragraph in paragraphs:
        pieces = split_large_paragraph(paragraph, max_chars)
        for piece in pieces:
            addition = len(piece) + (2 if current else 0)
            if current and current_len + addition > max_chars:
                chunks.append("\n\n".join(current).strip())
                current = [piece]
                current_len = len(piece)
            else:
                current.append(piece)
                current_len += addition
    if current:
        chunks.append("\n\n".join(current).strip())
    return [chunk for chunk in chunks if chunk]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Extract a PDF into Markdown pages, sections, and chunks with YAML frontmatter."
    )
    parser.add_argument("source", help="Path to the source PDF.")
    parser.add_argument(
        "--output",
        help="Output directory. Defaults to ./.pdf-extract/runs/<pdf-stem>.",
    )
    parser.add_argument(
        "--engine",
        choices=["auto", "pymupdf4llm", "fitz"],
        default="auto",
        help="Extraction engine. Default: auto.",
    )
    parser.add_argument(
        "--chunk-chars",
        type=int,
        default=6000,
        help="Approximate maximum characters per chunk file. Default: 6000.",
    )
    parser.add_argument(
        "--max-pages",
        type=int,
        help="Limit extraction to the first N pages for quick test runs.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Replace the output directory if it already exists.",
    )
    return parser.parse_args()


def extract_pages(
    pdf_path: Path, doc: fitz.Document, engine: str, max_pages: int | None
) -> tuple[str, list[PageRecord]]:
    if engine in {"auto", "pymupdf4llm"} and pymupdf4llm is not None:
        return "pymupdf4llm", extract_pages_with_pymupdf4llm(pdf_path, max_pages)
    if engine == "pymupdf4llm":
        raise SystemExit("error: pymupdf4llm is not installed in the selected Python environment")
    return "fitz", extract_pages_with_fitz(doc, max_pages)


def relative_posix(path: Path, base: Path) -> str:
    return path.relative_to(base).as_posix()


def render_toc_markdown(title: str, toc_records: list[TocRecord], output_dir: Path) -> str:
    lines = [f"# {title}", "", "## Table of Contents", ""]
    for record in toc_records:
        indent = "  " * max(record.level - 1, 0)
        link = f"./sections/{record.slug}.md"
        lines.append(f"{indent}- [{record.title}]({link}) (pp. {record.page_start}-{record.page_end})")
    lines.append("")
    return "\n".join(lines)


def render_output_readme(manifest: dict[str, Any]) -> str:
    return "\n".join(
        [
            f"# {manifest['title']}",
            "",
            f"- source: `{manifest['source_file']}`",
            f"- engine: `{manifest['engine']}`",
            f"- generated_at: `{manifest['generated_at']}`",
            f"- total_pages: `{manifest['page_count']}`",
            f"- extracted_pages: `{manifest['extracted_pages']}`",
            f"- sections: `{manifest['section_count']}`",
            f"- chunks: `{manifest['chunk_count']}`",
            "",
            "Generated files:",
            "",
            "- `master.md` - one concatenated markdown file with page markers",
            "- `toc.md` / `toc.json` - extracted outline",
            "- `pages/*.md` - per-page markdown with frontmatter",
            "- `sections/*.md` - outline-driven section files with frontmatter",
            "- `chunks/*.md` - chunked section markdown for RAG / embeddings",
            "- `manifest.json` - extraction metadata",
            "",
        ]
    )


def main() -> int:
    args = parse_args()
    source = Path(args.source).expanduser().resolve()
    if not source.exists():
        raise SystemExit(f"error: source PDF not found: {source}")
    if source.suffix.lower() != ".pdf":
        raise SystemExit(f"error: expected a PDF file, got: {source}")

    script_dir = Path(__file__).resolve().parent
    repo_root = script_dir.parent

    if args.output:
        output_dir = Path(args.output).expanduser().resolve()
    else:
        output_dir = (repo_root / ".pdf-extract" / "runs" / slugify(source.stem)).resolve()

    if output_dir.exists():
        if not args.overwrite:
            raise SystemExit(
                f"error: output directory already exists: {output_dir}\n"
                "pass --overwrite to replace it."
            )
        shutil.rmtree(output_dir)

    pages_dir = output_dir / "pages"
    sections_dir = output_dir / "sections"
    chunks_dir = output_dir / "chunks"
    output_dir.mkdir(parents=True, exist_ok=True)
    pages_dir.mkdir(parents=True, exist_ok=True)
    sections_dir.mkdir(parents=True, exist_ok=True)
    chunks_dir.mkdir(parents=True, exist_ok=True)

    doc = fitz.open(source)
    limited_page_count = min(doc.page_count, args.max_pages or doc.page_count)

    engine_used, page_records = extract_pages(source, doc, args.engine, args.max_pages)
    page_records = [record for record in page_records if record.page_number <= limited_page_count]
    page_map = {record.page_number: record for record in page_records}

    metadata = doc.metadata or {}
    title = metadata.get("title") or source.stem
    author = metadata.get("author") or ""
    generated_at = datetime.now(timezone.utc).isoformat()

    toc_records = build_toc_records(doc, args.max_pages)
    if not toc_records:
        toc_records = [
            TocRecord(
                index=1,
                level=1,
                title=title,
                page_start=1,
                page_end=limited_page_count,
                path=[title],
                next_title=None,
                next_page=None,
                slug="001-document",
            )
        ]

    manifest: dict[str, Any] = {
        "title": title,
        "author": author,
        "source_file": str(source),
        "output_dir": str(output_dir),
        "engine": engine_used,
        "page_count": doc.page_count,
        "extracted_pages": limited_page_count,
        "generated_at": generated_at,
        "chunk_chars": args.chunk_chars,
    }

    page_index_rows: list[dict[str, Any]] = []
    for page in page_records:
        page_file = pages_dir / f"{page.page_number:04d}.md"
        frontmatter = {
            "title": title,
            "author": author,
            "source_file": str(source),
            "page_number": page.page_number,
            "page_count": limited_page_count,
            "engine": engine_used,
            "generated_at": generated_at,
            "toc_items": page.toc_items,
        }
        write_markdown(page_file, frontmatter, page.text or "")
        page_index_rows.append(
            {
                "page_number": page.page_number,
                "file": relative_posix(page_file, output_dir),
                "toc_items": page.toc_items,
                "char_count": len(page.text),
            }
        )

    section_index_rows: list[dict[str, Any]] = []
    section_bodies: list[str] = []
    section_jsonl_rows: list[dict[str, Any]] = []
    chunk_index_rows: list[dict[str, Any]] = []
    chunk_jsonl_rows: list[dict[str, Any]] = []

    for record in toc_records:
        body = build_section_body(record, page_map)
        section_file = sections_dir / f"{record.slug}.md"
        frontmatter = {
            "title": title,
            "author": author,
            "source_file": str(source),
            "section_index": record.index,
            "section_title": record.title,
            "section_level": record.level,
            "section_path": record.path,
            "page_start": record.page_start,
            "page_end": record.page_end,
            "engine": engine_used,
            "generated_at": generated_at,
        }
        write_markdown(section_file, frontmatter, body or f"# {record.title}\n")
        section_index_rows.append(
            {
                "section_index": record.index,
                "title": record.title,
                "level": record.level,
                "page_start": record.page_start,
                "page_end": record.page_end,
                "path": record.path,
                "file": relative_posix(section_file, output_dir),
                "char_count": len(body),
            }
        )
        section_jsonl_rows.append(
            {
                "section_index": record.index,
                "title": record.title,
                "level": record.level,
                "page_start": record.page_start,
                "page_end": record.page_end,
                "path": record.path,
                "file": relative_posix(section_file, output_dir),
                "text": body,
            }
        )
        section_bodies.append(f"<!-- section:{record.index:03d} {record.title} -->\n\n{body.strip()}")

        for chunk_index, chunk_text in enumerate(chunk_markdown(body, args.chunk_chars), start=1):
            chunk_file = chunks_dir / f"{record.slug}-chunk-{chunk_index:03d}.md"
            chunk_frontmatter = {
                "title": title,
                "author": author,
                "source_file": str(source),
                "section_index": record.index,
                "section_title": record.title,
                "section_path": record.path,
                "chunk_index": chunk_index,
                "page_start": record.page_start,
                "page_end": record.page_end,
                "chunk_chars": len(chunk_text),
                "engine": engine_used,
                "generated_at": generated_at,
            }
            write_markdown(chunk_file, chunk_frontmatter, chunk_text)
            chunk_index_rows.append(
                {
                    "section_index": record.index,
                    "chunk_index": chunk_index,
                    "section_title": record.title,
                    "page_start": record.page_start,
                    "page_end": record.page_end,
                    "file": relative_posix(chunk_file, output_dir),
                    "char_count": len(chunk_text),
                }
            )
            chunk_jsonl_rows.append(
                {
                    "section_index": record.index,
                    "chunk_index": chunk_index,
                    "section_title": record.title,
                    "page_start": record.page_start,
                    "page_end": record.page_end,
                    "file": relative_posix(chunk_file, output_dir),
                    "text": chunk_text,
                }
            )

    manifest["section_count"] = len(section_index_rows)
    manifest["chunk_count"] = len(chunk_index_rows)

    toc_json_rows = [
        {
            "section_index": record.index,
            "level": record.level,
            "title": record.title,
            "page_start": record.page_start,
            "page_end": record.page_end,
            "path": record.path,
            "file": f"sections/{record.slug}.md",
        }
        for record in toc_records
    ]

    master_file = output_dir / "master.md"
    master_frontmatter = {
        "title": title,
        "author": author,
        "source_file": str(source),
        "engine": engine_used,
        "page_count": doc.page_count,
        "extracted_pages": limited_page_count,
        "section_count": len(section_index_rows),
        "chunk_count": len(chunk_index_rows),
        "generated_at": generated_at,
    }
    write_markdown(master_file, master_frontmatter, "\n\n".join(section_bodies))

    toc_md = render_toc_markdown(title, toc_records, output_dir)
    (output_dir / "toc.md").write_text(toc_md, encoding="utf-8")
    (output_dir / "README.md").write_text(render_output_readme(manifest), encoding="utf-8")

    write_json(output_dir / "manifest.json", manifest)
    write_json(output_dir / "toc.json", toc_json_rows)
    write_json(output_dir / "pages.json", page_index_rows)
    write_json(output_dir / "sections.json", section_index_rows)
    write_json(output_dir / "chunks.json", chunk_index_rows)
    write_jsonl(output_dir / "sections.jsonl", section_jsonl_rows)
    write_jsonl(output_dir / "chunks.jsonl", chunk_jsonl_rows)

    eprint(f"extracted {limited_page_count} pages with {engine_used} into {output_dir}")
    eprint(f"wrote {len(section_index_rows)} sections and {len(chunk_index_rows)} chunks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
