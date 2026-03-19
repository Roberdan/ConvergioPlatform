# Troubleshooting

## Problem: Dashboard shows blank page after rebuild

**Symptom:** White/black page, no content rendered
**Cause:** Maranello IIFE bundle not loading (404 on maranello.min.js)
**Fix:** Ensure MaranelloLuceDesign is installed: `npm install` in ConvergioPlatform root, or check node_modules path in index.html

## Problem: WebSocket connections fail in dashboard

**Symptom:** Terminal, Brain, or real-time widgets show "disconnected"
**Cause:** Daemon not running or WS endpoint not available
**Fix:** Start daemon via `dashboard/start.sh`. Check port 8420 is listening: `lsof -i :8420`
