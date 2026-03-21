#!/usr/bin/env bash
# convergio-aliases.sh — Shell aliases for Convergio CLI
# Source from ~/.zshrc: source ~/GitHub/ConvergioPlatform/scripts/platform/convergio-aliases.sh

export CONVERGIO_PLATFORM_DIR="${CONVERGIO_PLATFORM_DIR:-$HOME/GitHub/ConvergioPlatform}"
export PATH="$CONVERGIO_PLATFORM_DIR/scripts/platform:$PATH"

convergioOn()  { convergio on; }
convergioOff() { convergio off; }
