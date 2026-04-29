#!/usr/bin/env python3
"""
Extract local source documents (Markdown, HTML, TXT, EPUB) into a run layout
that mirrors the repo-local PDF extractor closely enough for source-pack use.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import zipfile
from dataclasses import dataclass
from datetime import datetime, timezone
from html import unescape
from html.parser import HTMLParser
from pathlib import Path
from typing import Any
from xml.etree import ElementTree as ET


@dataclass
class SourceUnit:
    index: int
    title: str
    source_path: str
    text: str


@dataclass
class SectionRecord:
    index: int
    level: int
    title: str
    path: list[str]
    slug: str
    unit_start: int
    unit_end: int
    text: str


class BlockHTMLToMarkdown(HTMLParser):
    def __init__(self) -> None:
        super().__init__(convert_charrefs=True)
        self.blocks: list[str] = []
        self.current: list[str] = []
        self.mode: tuple[str, int] | None = None
        self.ignore_depth = 0
        self.in_pre = False
        self.pre_parts: list[str] = []
        self.title_text: str | None = None
        self._capture_title = False
        self._title_parts: list[str] = []

    def handle_starttag(self, tag: str, attrs: list[tuple[str, str | None]]) -> None:
        tag = tag.lower()
        if tag in {"script", "style"}:
            self.ignore_depth += 1
            return
        if self.ignore_depth:
            return
        if tag == "title":
            self._capture_title = True
            self._title_parts = []
            return
        if tag in {"h1", "h2", "h3", "h4", "h5", "h6"}:
            self._flush_current()
            self.mode = ("heading", int(tag[1]))
            self.current = []
            return
        if tag == "pre":
            self._flush_current()
            self.in_pre = True
            self.pre_parts = []
            return
        if tag == "li":
            self._flush_current()
            self.mode = ("list_item", 0)
            self.current = []
            return
        if tag in {"p", "div", "section", "article", "blockquote"}:
            self._flush_current()
            self.mode = ("paragraph", 0)
            self.current = []
            return
        if tag == "br":
            if self.in_pre:
                self.pre_parts.append("\n")
            else:
                self.current.append("\n")

    def handle_endtag(self, tag: str) -> None:
        tag = tag.lower()
        if tag in {"script", "style"}:
            if self.ignore_depth:
                self.ignore_depth -= 1
            return
        if self.ignore_depth:
            return
        if tag == "title":
            self._capture_title = False
            title = normalize_space("".join(self._title_parts))
            if title:
                self.title_text = title
            return
        if tag == "pre":
            self._flush_pre()
            return
        if tag in {"h1", "h2", "h3", "h4", "h5", "h6", "li", "p", "div", "section", "article", "blockquote"}:
            self._flush_current()

    def handle_data(self, data: str) -> None:
        if self.ignore_depth:
            return
        if self._capture_title:
            self._title_parts.append(data)
            return
        if self.in_pre:
            self.pre_parts.append(data)
        else:
            self.current.append(data)

    def _flush_pre(self) -> None:
        text = "".join(self.pre_parts).rstrip()
        self.in_pre = False
        self.pre_parts = []
        if text:
            self.blocks.append(f"```text\n{text}\n```")

    def _flush_current(self) -> None:
        if not self.current:
            self.mode = None
            return
        raw = "".join(self.current)
        self.current = []
        text = normalize_space(raw)
        mode = self.mode
        self.mode = None
        if not text:
            return
        if mode and mode[0] == "heading":
            level = mode[1]
            self.blocks.append(f"{'#' * level} {text}")
            return
        if mode and mode[0] == "list_item":
            self.blocks.append(f"- {text}")
            return
        self.blocks.append(text)


def normalize_space(text: str) -> str:
    lines = [re.sub(r"[ \t]+", " ", line).strip() for line in text.replace("\r\n", "\n").replace("\r", "\n").split("\n")]
    compact = "\n".join(line for line in lines if line)
    compact = re.sub(r"\n{3,}", "\n\n", compact)
    return compact.strip()


def slugify(value: str) -> str:
    slug = re.sub(r"[^a-z0-9]+", "-", value.lower())
    slug = slug.strip("-")
    return slug or "section"


def split_large_paragraph(paragraph: str, max_chars: int) -> list[str]:
    if len(paragraph) <= max_chars:
        return [paragraph]
    words = paragraph.split()
    if not words:
        return [paragraph]
    chunks: list[str] = []
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


def yaml_scalar(value: Any) -> str:
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, (int, float)):
        return str(value)
    return json.dumps(str(value), ensure_ascii=False)


def yaml_lines(data: Any, indent: int = 0) -> list[str]:
    prefix = " " * indent
    if isinstance(data, dict):
        lines: list[str] = []
        for key, value in data.items():
            if isinstance(value, (dict, list)):
                lines.append(f"{prefix}{key}:")
                lines.extend(yaml_lines(value, indent + 2))
            else:
                lines.append(f"{prefix}{key}: {yaml_scalar(value)}")
        return lines
    if isinstance(data, list):
        lines = []
        for value in data:
            if isinstance(value, (dict, list)):
                lines.append(f"{prefix}-")
                lines.extend(yaml_lines(value, indent + 2))
            else:
                lines.append(f"{prefix}- {yaml_scalar(value)}")
        return lines
    return [f"{prefix}{yaml_scalar(data)}"]


def write_markdown(path: Path, frontmatter: dict[str, Any], body: str) -> None:
    lines = ["---", *yaml_lines(frontmatter), "---", "", body.rstrip(), ""]
    path.write_text("\n".join(lines), encoding="utf-8")


def write_json(path: Path, data: Any) -> None:
    path.write_text(json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def write_jsonl(path: Path, rows: list[dict[str, Any]]) -> None:
    with path.open("w", encoding="utf-8") as handle:
        for row in rows:
            handle.write(json.dumps(row, ensure_ascii=False) + "\n")


def relative_posix(path: Path, base: Path) -> str:
    return path.relative_to(base).as_posix()


def html_to_markdown(text: str) -> tuple[str, str | None]:
    parser = BlockHTMLToMarkdown()
    parser.feed(text)
    parser.close()
    body = "\n\n".join(block for block in parser.blocks if block.strip())
    return body.strip(), parser.title_text


def read_markdown_source(source: Path) -> tuple[list[SourceUnit], str | None, str | None]:
    text = source.read_text(encoding="utf-8")
    return [SourceUnit(index=1, title=source.stem, source_path=source.name, text=normalize_space(text))], None, None


def read_text_source(source: Path) -> tuple[list[SourceUnit], str | None, str | None]:
    text = source.read_text(encoding="utf-8")
    return [SourceUnit(index=1, title=source.stem, source_path=source.name, text=normalize_space(text))], None, None


def read_html_source(source: Path) -> tuple[list[SourceUnit], str | None, str | None]:
    raw = source.read_text(encoding="utf-8")
    body, title = html_to_markdown(raw)
    unit_title = title or source.stem
    if body and not re.search(r"^#{1,6}\s+", body, re.MULTILINE):
        body = f"# {unit_title}\n\n{body}"
    return [SourceUnit(index=1, title=unit_title, source_path=source.name, text=body)], title, None


def resolve_epub_rootfile(book: zipfile.ZipFile) -> str:
    container_xml = book.read("META-INF/container.xml")
    root = ET.fromstring(container_xml)
    ns = {"c": "urn:oasis:names:tc:opendocument:xmlns:container"}
    rootfile = root.find(".//c:rootfile", ns)
    if rootfile is None:
        raise SystemExit("error: EPUB container.xml does not declare a rootfile")
    full_path = rootfile.attrib.get("full-path")
    if not full_path:
        raise SystemExit("error: EPUB rootfile is missing full-path")
    return full_path


def read_epub_source(source: Path) -> tuple[list[SourceUnit], str | None, str | None]:
    with zipfile.ZipFile(source) as book:
        rootfile = resolve_epub_rootfile(book)
        package = ET.fromstring(book.read(rootfile))
        ns = {
            "opf": "http://www.idpf.org/2007/opf",
            "dc": "http://purl.org/dc/elements/1.1/",
        }
        opf_dir = Path(rootfile).parent
        title = package.findtext(".//dc:title", default="", namespaces=ns) or source.stem
        author = package.findtext(".//dc:creator", default="", namespaces=ns) or None

        manifest: dict[str, tuple[str, str]] = {}
        for item in package.findall(".//opf:manifest/opf:item", ns):
            item_id = item.attrib.get("id")
            href = item.attrib.get("href")
            media_type = item.attrib.get("media-type", "")
            if item_id and href:
                manifest[item_id] = (href, media_type)

        units: list[SourceUnit] = []
        for index, itemref in enumerate(package.findall(".//opf:spine/opf:itemref", ns), start=1):
            ref = itemref.attrib.get("idref")
            if not ref or ref not in manifest:
                continue
            href, media_type = manifest[ref]
            if "html" not in media_type and "xhtml" not in media_type:
                continue
            member_path = (opf_dir / href).as_posix()
            raw = book.read(member_path).decode("utf-8", errors="ignore")
            body, html_title = html_to_markdown(raw)
            unit_title = html_title or Path(href).stem or f"unit-{index:03d}"
            if body and not re.search(r"^#{1,6}\s+", body, re.MULTILINE):
                body = f"# {unit_title}\n\n{body}"
            units.append(
                SourceUnit(
                    index=index,
                    title=unit_title,
                    source_path=member_path,
                    text=body.strip(),
                )
            )
        if not units:
            raise SystemExit("error: EPUB did not yield any HTML/XHTML spine documents")
        return units, title, author


def read_source(source: Path) -> tuple[list[SourceUnit], str | None, str | None]:
    suffix = source.suffix.lower()
    if suffix in {".md", ".markdown"}:
        return read_markdown_source(source)
    if suffix in {".txt"}:
        return read_text_source(source)
    if suffix in {".html", ".htm"}:
        return read_html_source(source)
    if suffix == ".epub":
        return read_epub_source(source)
    raise SystemExit(f"error: unsupported source-doc format: {source.suffix}")


def build_master_text(units: list[SourceUnit], title: str) -> str:
    parts: list[str] = []
    for unit in units:
        parts.append(f"<!-- source-unit:{unit.index:04d} {unit.source_path} -->")
        text = unit.text.strip()
        if text:
            parts.append(text)
    master = "\n\n".join(parts).strip()
    if not re.search(r"^#{1,6}\s+", master, re.MULTILINE):
        master = f"<!-- source-unit:0001 document -->\n\n# {title}\n\n{master}"
    return master


def build_sections(master_text: str, title: str) -> list[SectionRecord]:
    lines = master_text.splitlines()
    unit = 1
    sections: list[SectionRecord] = []
    current_title = title
    current_level = 1
    current_path = [title]
    current_start = 1
    current_lines: list[str] = []
    path_stack = [title]
    saw_heading = False

    def flush(current_unit: int) -> None:
        nonlocal current_lines, current_title, current_level, current_path, current_start, sections
        text = "\n".join(current_lines).strip()
        meaningful = "\n".join(
            line
            for line in current_lines
            if line.strip() and not re.match(r"^<!--\s*source-unit:\d+", line)
        ).strip()
        if not text or not meaningful:
            current_lines = []
            return
        index = len(sections) + 1
        slug = f"{index:03d}-{slugify(current_title)}"
        sections.append(
            SectionRecord(
                index=index,
                level=current_level,
                title=current_title,
                path=current_path[:],
                slug=slug,
                unit_start=current_start,
                unit_end=current_unit,
                text=text,
            )
        )
        current_lines = []

    heading_re = re.compile(r"^(#{1,6})\s+(.*?)\s*$")
    unit_re = re.compile(r"^<!--\s*source-unit:(\d+)")

    for line in lines:
        unit_match = unit_re.match(line)
        if unit_match:
            unit = int(unit_match.group(1))
        heading_match = heading_re.match(line)
        if heading_match:
            saw_heading = True
            flush(unit)
            level = len(heading_match.group(1))
            heading_title = heading_match.group(2).strip()
            path_stack = path_stack[: max(level - 1, 0)]
            path_stack.append(heading_title)
            current_title = heading_title
            current_level = level
            current_path = path_stack[:]
            current_start = unit
            current_lines = [line]
            continue
        current_lines.append(line)

    flush(unit)

    if not sections and master_text.strip():
        sections.append(
            SectionRecord(
                index=1,
                level=1,
                title=title,
                path=[title],
                slug="001-document",
                unit_start=1,
                unit_end=max(unit, 1),
                text=master_text.strip(),
            )
        )
    elif sections and not saw_heading:
        sections[0].title = title
        sections[0].path = [title]
        sections[0].slug = "001-document"
    return sections


def render_toc_markdown(title: str, records: list[SectionRecord]) -> str:
    lines = [f"# {title}", "", "## Table of Contents", ""]
    for record in records:
        indent = "  " * max(record.level - 1, 0)
        lines.append(
            f"{indent}- [{record.title}](./sections/{record.slug}.md) "
            f"(units {record.unit_start}-{record.unit_end})"
        )
    lines.append("")
    return "\n".join(lines)


def render_readme(manifest: dict[str, Any]) -> str:
    return "\n".join(
        [
            f"# {manifest['title']}",
            "",
            f"- source: `{manifest['source_file']}`",
            f"- engine: `{manifest['engine']}`",
            f"- generated_at: `{manifest['generated_at']}`",
            f"- source_units: `{manifest['source_unit_count']}`",
            f"- sections: `{manifest['section_count']}`",
            f"- chunks: `{manifest['chunk_count']}`",
            "",
            "Generated files:",
            "",
            "- `master.md` - one concatenated markdown file with source-unit markers",
            "- `toc.md` / `toc.json` - derived section outline",
            "- `pages/*.md` - per-source-unit markdown with frontmatter",
            "- `sections/*.md` - heading-driven section files with frontmatter",
            "- `chunks/*.md` - chunked section markdown for RAG / embeddings",
            "- `manifest.json` - extraction metadata",
            "",
        ]
    )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Extract a local Markdown, HTML, TXT, or EPUB source document into markdown sections and chunks."
    )
    parser.add_argument("source", help="Path to the source document.")
    parser.add_argument("--output", help="Output directory. Defaults to ./.pdf-extract/runs/<source-stem>.")
    parser.add_argument("--title", help="Optional title override.")
    parser.add_argument("--author", help="Optional author override.")
    parser.add_argument(
        "--chunk-chars",
        type=int,
        default=6000,
        help="Approximate maximum characters per chunk file. Default: 6000.",
    )
    parser.add_argument(
        "--overwrite",
        action="store_true",
        help="Replace the output directory if it already exists.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    source = Path(args.source).expanduser().resolve()
    if not source.exists():
        raise SystemExit(f"error: source document not found: {source}")

    script_dir = Path(__file__).resolve().parent
    repo_root = script_dir.parent
    if args.output:
        output_dir = Path(args.output).expanduser().resolve()
    else:
        output_dir = (repo_root / ".pdf-extract" / "runs" / slugify(source.stem)).resolve()

    if output_dir.exists():
        if not args.overwrite:
            raise SystemExit(
                f"error: output directory already exists: {output_dir}\npass --overwrite to replace it."
            )
        shutil.rmtree(output_dir)

    pages_dir = output_dir / "pages"
    sections_dir = output_dir / "sections"
    chunks_dir = output_dir / "chunks"
    output_dir.mkdir(parents=True, exist_ok=True)
    pages_dir.mkdir(parents=True, exist_ok=True)
    sections_dir.mkdir(parents=True, exist_ok=True)
    chunks_dir.mkdir(parents=True, exist_ok=True)

    units, detected_title, detected_author = read_source(source)
    title = args.title or detected_title or source.stem
    author = args.author or detected_author or ""
    generated_at = datetime.now(timezone.utc).isoformat()
    engine = "source-doc"

    page_index_rows: list[dict[str, Any]] = []
    for unit in units:
        page_file = pages_dir / f"{unit.index:04d}.md"
        frontmatter = {
            "title": title,
            "author": author,
            "source_file": str(source),
            "source_unit_index": unit.index,
            "source_unit_title": unit.title,
            "source_unit_count": len(units),
            "source_unit_path": unit.source_path,
            "engine": engine,
            "generated_at": generated_at,
        }
        write_markdown(page_file, frontmatter, unit.text or f"# {unit.title}\n")
        page_index_rows.append(
            {
                "source_unit_index": unit.index,
                "title": unit.title,
                "path": unit.source_path,
                "file": relative_posix(page_file, output_dir),
                "char_count": len(unit.text),
            }
        )

    master_text = build_master_text(units, title)
    sections = build_sections(master_text, title)

    section_index_rows: list[dict[str, Any]] = []
    section_jsonl_rows: list[dict[str, Any]] = []
    chunk_index_rows: list[dict[str, Any]] = []
    chunk_jsonl_rows: list[dict[str, Any]] = []
    master_bodies: list[str] = []

    for section in sections:
        section_file = sections_dir / f"{section.slug}.md"
        frontmatter = {
            "title": title,
            "author": author,
            "source_file": str(source),
            "section_index": section.index,
            "section_title": section.title,
            "section_level": section.level,
            "section_path": section.path,
            "source_unit_start": section.unit_start,
            "source_unit_end": section.unit_end,
            "engine": engine,
            "generated_at": generated_at,
        }
        write_markdown(section_file, frontmatter, section.text)
        section_index_rows.append(
            {
                "section_index": section.index,
                "title": section.title,
                "level": section.level,
                "path": section.path,
                "source_unit_start": section.unit_start,
                "source_unit_end": section.unit_end,
                "file": relative_posix(section_file, output_dir),
                "char_count": len(section.text),
            }
        )
        section_jsonl_rows.append(
            {
                "section_index": section.index,
                "title": section.title,
                "level": section.level,
                "path": section.path,
                "source_unit_start": section.unit_start,
                "source_unit_end": section.unit_end,
                "file": relative_posix(section_file, output_dir),
                "text": section.text,
            }
        )
        master_bodies.append(f"<!-- section:{section.index:03d} {section.title} -->\n\n{section.text.strip()}")

        for chunk_index, chunk_text in enumerate(chunk_markdown(section.text, args.chunk_chars), start=1):
            chunk_file = chunks_dir / f"{section.slug}-chunk-{chunk_index:03d}.md"
            chunk_frontmatter = {
                "title": title,
                "author": author,
                "source_file": str(source),
                "section_index": section.index,
                "section_title": section.title,
                "section_path": section.path,
                "chunk_index": chunk_index,
                "source_unit_start": section.unit_start,
                "source_unit_end": section.unit_end,
                "chunk_chars": len(chunk_text),
                "engine": engine,
                "generated_at": generated_at,
            }
            write_markdown(chunk_file, chunk_frontmatter, chunk_text)
            chunk_index_rows.append(
                {
                    "section_index": section.index,
                    "chunk_index": chunk_index,
                    "section_title": section.title,
                    "source_unit_start": section.unit_start,
                    "source_unit_end": section.unit_end,
                    "file": relative_posix(chunk_file, output_dir),
                    "char_count": len(chunk_text),
                }
            )
            chunk_jsonl_rows.append(
                {
                    "section_index": section.index,
                    "chunk_index": chunk_index,
                    "section_title": section.title,
                    "source_unit_start": section.unit_start,
                    "source_unit_end": section.unit_end,
                    "file": relative_posix(chunk_file, output_dir),
                    "text": chunk_text,
                }
            )

    manifest: dict[str, Any] = {
        "title": title,
        "author": author,
        "source_file": str(source),
        "output_dir": str(output_dir),
        "engine": engine,
        "source_unit_count": len(units),
        "generated_at": generated_at,
        "chunk_chars": args.chunk_chars,
        "section_count": len(section_index_rows),
        "chunk_count": len(chunk_index_rows),
    }

    master_file = output_dir / "master.md"
    master_frontmatter = {
        "title": title,
        "author": author,
        "source_file": str(source),
        "engine": engine,
        "source_unit_count": len(units),
        "section_count": len(section_index_rows),
        "chunk_count": len(chunk_index_rows),
        "generated_at": generated_at,
    }
    write_markdown(master_file, master_frontmatter, "\n\n".join(master_bodies))

    toc_rows = [
        {
            "section_index": section.index,
            "level": section.level,
            "title": section.title,
            "path": section.path,
            "source_unit_start": section.unit_start,
            "source_unit_end": section.unit_end,
            "file": f"sections/{section.slug}.md",
        }
        for section in sections
    ]

    (output_dir / "README.md").write_text(render_readme(manifest), encoding="utf-8")
    (output_dir / "toc.md").write_text(render_toc_markdown(title, sections), encoding="utf-8")
    write_json(output_dir / "manifest.json", manifest)
    write_json(output_dir / "toc.json", toc_rows)
    write_json(output_dir / "pages.json", page_index_rows)
    write_json(output_dir / "sections.json", section_index_rows)
    write_json(output_dir / "chunks.json", chunk_index_rows)
    write_jsonl(output_dir / "sections.jsonl", section_jsonl_rows)
    write_jsonl(output_dir / "chunks.jsonl", chunk_jsonl_rows)

    print(f"extracted {len(units)} source units into {output_dir}")
    print(f"wrote {len(section_index_rows)} sections and {len(chunk_index_rows)} chunks")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
