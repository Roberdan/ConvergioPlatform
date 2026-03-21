#!/usr/bin/env bash
# convergio-ingest-helpers.sh — Per-format conversion helpers for convergio-ingest.sh
# Sourced by convergio-ingest.sh. All tools optional — graceful fallback with warning.
set -euo pipefail

# --------------------------------------------------------------------------- #
# Utility                                                                       #
# --------------------------------------------------------------------------- #

_warn() { echo "WARN: $*" >&2; }
_info() { echo "INFO: $*" >&2; }
_has() { command -v "$1" &>/dev/null; }

# --------------------------------------------------------------------------- #
# PDF                                                                           #
# --------------------------------------------------------------------------- #

ingest_pdf() {
  local src="$1" dest="$2"
  local out="${dest}/$(basename "${src%.pdf}").md"

  if _has pdftotext; then
    _info "PDF: pdftotext <- $(basename "$src")"
    pdftotext -layout "$src" - | _wrap_md "$(basename "$src")" > "$out"
  elif _has python3 && python3 -c "import fitz" 2>/dev/null; then
    _info "PDF: pymupdf <- $(basename "$src")"
    python3 - "$src" <<'PYEOF' | _wrap_md "$(basename "$src")" > "$out"
import sys, fitz
doc = fitz.open(sys.argv[1])
for page in doc:
    print(page.get_text())
PYEOF
  else
    _warn "PDF skipped (need pdftotext or pymupdf): $(basename "$src")"
    return 0
  fi
}

# --------------------------------------------------------------------------- #
# DOCX                                                                          #
# --------------------------------------------------------------------------- #

ingest_docx() {
  local src="$1" dest="$2"
  local out="${dest}/$(basename "${src%.docx}").md"

  if _has pandoc; then
    _info "DOCX: pandoc <- $(basename "$src")"
    pandoc --from=docx --to=markdown --wrap=none -o "$out" "$src"
  elif _has python3 && python3 -c "import docx" 2>/dev/null; then
    _info "DOCX: python-docx <- $(basename "$src")"
    python3 - "$src" <<'PYEOF' | _wrap_md "$(basename "$src")" > "$out"
import sys
from docx import Document
doc = Document(sys.argv[1])
for para in doc.paragraphs:
    print(para.text)
PYEOF
  else
    _warn "DOCX skipped (need pandoc or python-docx): $(basename "$src")"
    return 0
  fi
}

# --------------------------------------------------------------------------- #
# XLSX / CSV                                                                    #
# --------------------------------------------------------------------------- #

ingest_xlsx() {
  local src="$1" dest="$2"
  local base; base="$(basename "${src}")"
  local out="${dest}/${base%.*}.md"

  if _has python3 && python3 -c "import openpyxl" 2>/dev/null; then
    _info "XLSX: openpyxl <- $(basename "$src")"
    python3 - "$src" <<'PYEOF' | _wrap_md "$base" > "$out"
import sys, openpyxl
wb = openpyxl.load_workbook(sys.argv[1], read_only=True, data_only=True)
for sheet in wb.worksheets:
    print(f"\n## Sheet: {sheet.title}\n")
    for row in sheet.iter_rows(values_only=True):
        cells = [str(c) if c is not None else '' for c in row]
        print('| ' + ' | '.join(cells) + ' |')
PYEOF
  elif _has in2csv && _has csvlook; then
    _info "XLSX: csvkit <- $(basename "$src")"
    in2csv "$src" | csvlook | _wrap_md "$base" > "$out"
  else
    _warn "XLSX skipped (need openpyxl or csvkit): $(basename "$src")"
    return 0
  fi
}

ingest_csv() {
  local src="$1" dest="$2"
  local base; base="$(basename "$src")"
  local out="${dest}/${base%.csv}.md"

  if _has csvlook; then
    _info "CSV: csvlook <- $base"
    csvlook "$src" | _wrap_md "$base" > "$out"
  elif _has python3; then
    _info "CSV: python csv <- $base"
    python3 - "$src" <<'PYEOF' | _wrap_md "$base" > "$out"
import sys, csv
with open(sys.argv[1], newline='') as f:
    for row in csv.reader(f):
        print('| ' + ' | '.join(row) + ' |')
PYEOF
  else
    _warn "CSV skipped (need csvlook or python3): $base"
    return 0
  fi
}

# --------------------------------------------------------------------------- #
# PPTX                                                                          #
# --------------------------------------------------------------------------- #

ingest_pptx() {
  local src="$1" dest="$2"
  local out="${dest}/$(basename "${src%.pptx}").md"

  if _has python3 && python3 -c "from pptx import Presentation" 2>/dev/null; then
    _info "PPTX: python-pptx <- $(basename "$src")"
    python3 - "$src" <<'PYEOF' | _wrap_md "$(basename "$src")" > "$out"
import sys
from pptx import Presentation
prs = Presentation(sys.argv[1])
for i, slide in enumerate(prs.slides, 1):
    print(f"\n## Slide {i}\n")
    for shape in slide.shapes:
        if hasattr(shape, 'text') and shape.text.strip():
            print(shape.text)
PYEOF
  else
    _warn "PPTX skipped (need python-pptx): $(basename "$src")"
    return 0
  fi
}

# --------------------------------------------------------------------------- #
# URL                                                                            #
# --------------------------------------------------------------------------- #

ingest_url() {
  local url="$1" dest="$2"
  # Sanitize URL to safe filename
  local safe; safe="$(echo "$url" | sed 's|https\?://||;s|[^a-zA-Z0-9._-]|_|g' | cut -c1-80)"
  local out="${dest}/${safe}.md"

  if _has trafilatura; then
    _info "URL: trafilatura <- $url"
    trafilatura --url "$url" --output-format markdown 2>/dev/null > "$out" \
      || { _warn "trafilatura failed for: $url"; rm -f "$out"; return 0; }
  elif _has curl && _has html2text; then
    _info "URL: curl+html2text <- $url"
    curl -fsSL --max-time 30 "$url" 2>/dev/null \
      | html2text --ignore-links 2>/dev/null \
      | _wrap_md "$url" > "$out" \
      || { _warn "curl+html2text failed for: $url"; rm -f "$out"; return 0; }
  else
    _warn "URL skipped (need trafilatura or curl+html2text): $url"
    return 0
  fi
}

# --------------------------------------------------------------------------- #
# Image (copy as-is for Claude vision)                                          #
# --------------------------------------------------------------------------- #

ingest_image() {
  local src="$1" dest="$2"
  _info "Image: copy <- $(basename "$src")"
  cp "$src" "${dest}/$(basename "$src")"
}

# --------------------------------------------------------------------------- #
# Markdown / plain text (copy)                                                  #
# --------------------------------------------------------------------------- #

ingest_text() {
  local src="$1" dest="$2"
  local base; base="$(basename "$src")"
  local out="${dest}/${base}"
  # Ensure .md extension for plain text
  [[ "$src" == *.txt ]] && out="${dest}/${base%.txt}.md"
  _info "Text: copy <- $base"
  cp "$src" "$out"
}

# --------------------------------------------------------------------------- #
# Wrap raw text output in minimal markdown with a heading                       #
# --------------------------------------------------------------------------- #

_wrap_md() {
  local title="$1"
  printf '# %s\n\n' "$title"
  cat
}
