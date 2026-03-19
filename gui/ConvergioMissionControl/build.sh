#!/usr/bin/env bash
set -euo pipefail
# build.sh — Build ConvergioMissionControl SwiftUI app bundle

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
APP_NAME="ConvergioMissionControl"
BUILD_DIR="$SCRIPT_DIR/.build"
BUNDLE_DIR="$SCRIPT_DIR/build"

cleanup() {
    echo "Build finished."
}
trap cleanup EXIT

echo "=== Building $APP_NAME ==="

# 1. Build Swift package (release)
echo "[1/4] swift build -c release..."
cd "$SCRIPT_DIR"
swift build -c release

SWIFT_BIN="$BUILD_DIR/release/$APP_NAME"
if [[ ! -f "$SWIFT_BIN" ]]; then
    echo "ERROR: Binary not found at $SWIFT_BIN"
    exit 1
fi

# 2. Create .app bundle (SPM does not produce .app)
echo "[2/4] Creating app bundle..."
APP_DIR="$BUNDLE_DIR/$APP_NAME.app/Contents/MacOS"
RESOURCES_DIR="$BUNDLE_DIR/$APP_NAME.app/Contents/Resources"
mkdir -p "$APP_DIR" "$RESOURCES_DIR"

cp "$SWIFT_BIN" "$APP_DIR/$APP_NAME"

# Copy entitlements for reference
ENTITLEMENTS="$SCRIPT_DIR/$APP_NAME.entitlements"
if [[ -f "$ENTITLEMENTS" ]]; then
    cp "$ENTITLEMENTS" "$RESOURCES_DIR/"
fi

# Create Info.plist
cat > "$BUNDLE_DIR/$APP_NAME.app/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>ConvergioMissionControl</string>
    <key>CFBundleIdentifier</key>
    <string>io.convergio.mission-control</string>
    <key>CFBundleName</key>
    <string>ConvergioMissionControl</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
    <key>CFBundleShortVersionString</key>
    <string>0.1.0</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSUIElement</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.developer-tools</string>
</dict>
</plist>
PLIST

# 3. Code sign with entitlements
echo "[3/4] Code signing..."
if [[ -f "$ENTITLEMENTS" ]]; then
    codesign --deep --force --entitlements "$ENTITLEMENTS" \
        -s - "$BUNDLE_DIR/$APP_NAME.app"
else
    codesign --deep --force -s - "$BUNDLE_DIR/$APP_NAME.app"
fi

# 4. Verify
echo "[4/4] Verifying..."
if [[ ! -d "$BUNDLE_DIR/$APP_NAME.app" ]]; then
    echo "ERROR: App bundle not created"
    exit 1
fi

echo ""
echo "=== SUCCESS ==="
echo "App: $BUNDLE_DIR/$APP_NAME.app"
echo "Bin: $APP_DIR/$APP_NAME"
