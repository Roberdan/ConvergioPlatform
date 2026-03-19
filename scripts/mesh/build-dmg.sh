#!/usr/bin/env bash
set -euo pipefail
# build-dmg.sh — Build ConvergioMesh.dmg with app + CLI

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BUILD_DIR="$PROJECT_ROOT/build"
APP_NAME="ConvergioMesh"

echo "=== Building ConvergioMesh.dmg ==="

# 1. Build Rust CLI (release)
echo "[1/6] Building Rust CLI..."
cd "$PROJECT_ROOT"
cargo build --release
CLI_BIN="$PROJECT_ROOT/target/release/convergiomesh-cli"
[ -f "$CLI_BIN" ] || { echo "ERROR: CLI binary not found"; exit 1; }

# 2. Build SwiftUI menu bar app (release)
echo "[2/6] Building ConvergioMissionControl menu bar app..."
MISSION_CONTROL_DIR="$PROJECT_ROOT/gui/ConvergioMissionControl"
"$MISSION_CONTROL_DIR/build.sh"
SWIFT_BIN="$MISSION_CONTROL_DIR/build/ConvergioMissionControl.app/Contents/MacOS/ConvergioMissionControl"
[ -f "$SWIFT_BIN" ] || { echo "ERROR: MissionControl binary not found"; exit 1; }

# 3. Assemble DMG contents from MissionControl app + CLI
echo "[3/6] Assembling DMG contents..."
mkdir -p "$BUILD_DIR"
MCAPP_SRC="$MISSION_CONTROL_DIR/build/ConvergioMissionControl.app"
cp -R "$MCAPP_SRC" "$BUILD_DIR/$APP_NAME.app"

# Embed CLI binary alongside menu bar app
APP_DIR="$BUILD_DIR/$APP_NAME.app/Contents/MacOS"
cp "$CLI_BIN" "$APP_DIR/convergiomesh-cli"

# Copy app icon into bundle resources
RESOURCES_DIR="$BUILD_DIR/$APP_NAME.app/Contents/Resources"
ICON_SRC="$PROJECT_ROOT/resources/AppIcon.icns"
if [[ -f "$ICON_SRC" ]]; then
    cp "$ICON_SRC" "$RESOURCES_DIR/AppIcon.icns"
    echo "  App icon copied"
fi

# 4. Re-sign after embedding CLI
echo "[4/6] Code signing..."
ENTITLEMENTS="$MISSION_CONTROL_DIR/ConvergioMissionControl.entitlements"
if [[ -f "$ENTITLEMENTS" ]]; then
    codesign --deep --force --entitlements "$ENTITLEMENTS" \
        -s - "$BUILD_DIR/$APP_NAME.app"
else
    codesign --deep --force -s - "$BUILD_DIR/$APP_NAME.app"
fi

# 5. Create DMG
echo "[5/6] Creating DMG..."
# Check if create-dmg is available, fallback to hdiutil
if command -v create-dmg &>/dev/null; then
    create-dmg \
        --volname "$APP_NAME" \
        --window-pos 200 120 \
        --window-size 600 400 \
        --icon-size 100 \
        --icon "$APP_NAME.app" 150 190 \
        --app-drop-link 450 190 \
        --no-internet-enable \
        "$BUILD_DIR/$APP_NAME.dmg" \
        "$BUILD_DIR/$APP_NAME.app"
else
    echo "  create-dmg not found, using hdiutil..."
    STAGING="$BUILD_DIR/dmg-staging"
    rm -rf "$STAGING"
    mkdir -p "$STAGING"
    cp -R "$BUILD_DIR/$APP_NAME.app" "$STAGING/"
    ln -s /Applications "$STAGING/Applications"
    hdiutil create -volname "$APP_NAME" -srcfolder "$STAGING" \
        -ov -format UDZO "$BUILD_DIR/$APP_NAME.dmg"
    rm -rf "$STAGING"
fi

# 6. Verify
echo "[6/6] Verifying..."
[ -f "$BUILD_DIR/$APP_NAME.dmg" ] || { echo "ERROR: DMG not created"; exit 1; }
DMG_SIZE=$(du -h "$BUILD_DIR/$APP_NAME.dmg" | cut -f1)
echo ""
echo "=== SUCCESS ==="
echo "DMG: $BUILD_DIR/$APP_NAME.dmg ($DMG_SIZE)"
echo "App: $BUILD_DIR/$APP_NAME.app"
echo "CLI: $APP_DIR/convergiomesh-cli"
