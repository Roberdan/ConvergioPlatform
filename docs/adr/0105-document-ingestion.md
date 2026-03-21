# ADR 0105: Document Ingestion Engine

Status: Accepted | Date: 21 Mar 2026

## Context

Agents need to reason over external documents (PDFs, DOCX, spreadsheets, URLs, folders) as context for execution runs. Without a standard ingestion layer, each caller must handle conversion differently, producing inconsistent markdown and duplicate tool dependencies.

## Decision

Introduce `convergio-ingest.sh` as a standalone ingestion engine: it converts any supported source (PDF via `pdftotext`, DOCX/PPTX via `pandoc`, XLSX/CSV via `python3`, URLs via `trafilatura`, images via `tesseract`, text/markdown directly) into normalized markdown files in a caller-specified output directory. All conversion tools are optional — missing tools produce a `console.warn`-style stderr warning and skip that format gracefully. The `--context` flag on `convergio-run-ops.sh` accepts a source path, runs ingestion, and attaches the output to the execution run's `context_files` column.

## Consequences

- Positive: uniform markdown output for all formats; agents receive structured context without format handling; optional deps mean zero install friction
- Negative: quality degrades gracefully when tools are absent (e.g. curl-only URL vs trafilatura); image OCR requires `tesseract` separately
- Install guide: `brew install poppler pandoc tesseract` + `pip install trafilatura openpyxl`
