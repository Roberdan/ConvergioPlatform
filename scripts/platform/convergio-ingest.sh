#!/usr/bin/env bash
# convergio-ingest.sh — Document ingestion: converts files/URLs/folders to structured markdown.
# Supported: PDF, DOCX, XLSX, CSV, PPTX, images, URLs, markdown/text, folders (recursive).
# All conversion tools optional — graceful fallback with warning if missing.
#
# Usage: convergio-ingest.sh <source> <output_dir>
#   source       File path, URL (http/https), or folder path
#   output_dir   Directory where markdown output is written (created if absent)
#
# Examples:
#   convergio-ingest.sh report.pdf ./ingested/
#   convergio-ingest.sh https://example.com/page ./ingested/
#   convergio-ingest.sh ./docs-folder/ ./ingested/
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HELPERS="${SCRIPT_DIR}/convergio-ingest-helpers.sh"

# --------------------------------------------------------------------------- #
# Cleanup                                                                       #
# --------------------------------------------------------------------------- #

_TMPDIR=""
cleanup() {
  if [[ -n "${_TMPDIR:-}" && -d "${_TMPDIR:-}" ]]; then
    rm -rf "$_TMPDIR"
  fi
}
trap cleanup EXIT

# --------------------------------------------------------------------------- #
# Helpers                                                                       #
# --------------------------------------------------------------------------- #

usage() {
  sed -n '2,13s/^# \{0,1\}//p' "${BASH_SOURCE[0]}"
  exit 0
}

die() { echo "ERROR: $*" >&2; exit 1; }

_is_url() { [[ "$1" =~ ^https?:// ]]; }

# Load per-format helpers (sourced so they share this shell's PATH/env)
_load_helpers() {
  [[ -f "$HELPERS" ]] || die "Missing helpers file: $HELPERS"
  # shellcheck source=convergio-ingest-helpers.sh
  source "$HELPERS"
}

# --------------------------------------------------------------------------- #
# Dispatch single file by extension                                             #
# --------------------------------------------------------------------------- #

ingest_file() {
  local src="$1" dest="$2"
  local ext="${src##*.}"
  ext="$(echo "$ext" | tr '[:upper:]' '[:lower:]')"  # lowercase (bash 3 compat)

  case "$ext" in
    pdf)                ingest_pdf   "$src" "$dest" ;;
    docx)               ingest_docx  "$src" "$dest" ;;
    xlsx|xls)           ingest_xlsx  "$src" "$dest" ;;
    csv)                ingest_csv   "$src" "$dest" ;;
    pptx|ppt)           ingest_pptx  "$src" "$dest" ;;
    jpg|jpeg|png|gif|webp|bmp|tiff|svg)
                        ingest_image "$src" "$dest" ;;
    md|markdown|txt|rst)
                        ingest_text  "$src" "$dest" ;;
    *)
      echo "WARN: Unknown format, skipping: $(basename "$src")" >&2
      ;;
  esac
}

# --------------------------------------------------------------------------- #
# Folder recursion                                                              #
# --------------------------------------------------------------------------- #

ingest_folder() {
  local src="$1" dest="$2"
  local processed=0 skipped=0

  echo "INFO: Recursing folder: $src" >&2

  # Process each file; skip hidden files and directories
  while IFS= read -r -d '' filepath; do
    [[ -f "$filepath" ]] || continue
    # Preserve relative sub-directory structure under dest
    local rel="${filepath#"${src%/}/"}"
    local subdir="${rel%/*}"
    local file_dest="$dest"
    if [[ "$subdir" != "$rel" ]]; then
      file_dest="${dest}/${subdir}"
      mkdir -p "$file_dest"
    fi
    ingest_file "$filepath" "$file_dest" && (( processed++ )) || (( skipped++ ))
  done < <(find "$src" -not -path '*/.*' -type f -print0 | sort -z)

  echo "INFO: Folder done — processed=${processed} skipped=${skipped}" >&2
}

# --------------------------------------------------------------------------- #
# Main                                                                          #
# --------------------------------------------------------------------------- #

main() {
  local source="" output_dir=""

  # Parse args
  while [[ $# -gt 0 ]]; do
    case "$1" in
      -h|--help) usage ;;
      -*)        die "Unknown option: $1" ;;
      *)
        if [[ -z "$source" ]]; then
          source="$1"
        elif [[ -z "$output_dir" ]]; then
          output_dir="$1"
        else
          die "Too many arguments"
        fi
        ;;
    esac
    shift
  done

  [[ -n "$source" ]]     || die "Missing <source>. Run with --help for usage."
  [[ -n "$output_dir" ]] || die "Missing <output_dir>. Run with --help for usage."

  mkdir -p "$output_dir"
  _load_helpers

  if _is_url "$source"; then
    ingest_url "$source" "$output_dir"
  elif [[ -d "$source" ]]; then
    ingest_folder "$source" "$output_dir"
  elif [[ -f "$source" ]]; then
    ingest_file "$source" "$output_dir"
  else
    die "Source not found (not a file, directory, or URL): $source"
  fi

  echo "INFO: Ingestion complete -> $output_dir" >&2
}

main "$@"
