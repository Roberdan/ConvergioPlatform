#!/usr/bin/env bash
# convergio-daemon-ops.sh — Daemon lifecycle management
# Sourced by convergio. PLATFORM_DIR, DAEMON_URL, _daemon_ok(), color vars must be set.
set -euo pipefail

cmd_daemon() {
  local plist="$PLATFORM_DIR/config/com.convergio.daemon.plist"
  local la_dir="$HOME/Library/LaunchAgents"
  local la_path="$la_dir/com.convergio.daemon.plist"

  case "${1:-status}" in
    start)
      if _daemon_ok; then
        echo -e "${G}Daemon already running${N}"
      else
        bash "$PLATFORM_DIR/daemon/start.sh" &
        sleep 2
        _daemon_ok && echo -e "${G}Daemon started${N}" || echo -e "${R}Failed to start${N}"
      fi ;;
    stop)
      pkill -f "claude-core" 2>/dev/null && echo -e "${Y}Daemon stopped${N}" || echo "Not running" ;;
    restart)
      cmd_daemon stop; sleep 1; cmd_daemon start ;;
    install)
      mkdir -p "$la_dir"
      cp "$plist" "$la_path"
      launchctl load "$la_path" 2>/dev/null
      echo -e "${G}Daemon installed as LaunchAgent — starts on boot${N}"
      echo "  Plist: $la_path" ;;
    uninstall)
      launchctl unload "$la_path" 2>/dev/null
      rm -f "$la_path"
      echo -e "${Y}Daemon LaunchAgent removed${N}" ;;
    status)
      _daemon_ok && echo -e "${G}Daemon: running on $DAEMON_URL${N}" || echo -e "${R}Daemon: stopped${N}"
      test -f "$la_path" && echo "  LaunchAgent: installed (auto-start on boot)" || echo "  LaunchAgent: not installed"
      local pid
      pid=$(pgrep -f "claude-core" 2>/dev/null)
      [ -n "$pid" ] && echo "  PID: $pid" ;;
    logs)
      tail -50 "$HOME/.claude/data/convergio-daemon.log" 2>/dev/null || echo "No logs" ;;
    menubar)
      local swift_src="$PLATFORM_DIR/scripts/platform/convergio-menubar.swift"
      local bin="$PLATFORM_DIR/scripts/platform/convergio-menubar"
      if [ ! -x "$bin" ] || [ "$swift_src" -nt "$bin" ]; then
        echo "Building menu bar app..."
        swiftc "$swift_src" -o "$bin" -framework Cocoa 2>&1 \
          && echo -e "${G}Built${N}" \
          || { echo -e "${R}Build failed${N}"; return 1; }
      fi
      echo "Starting menu bar icon..."
      "$bin" &
      disown
      echo -e "${G}Menu bar icon active${N}" ;;
    *)
      echo "convergio daemon [start|stop|restart|install|uninstall|status|logs|menubar]" ;;
  esac
}
