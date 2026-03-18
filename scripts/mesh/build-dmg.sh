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

# 2. Build SwiftUI app (release)
echo "[2/6] Building SwiftUI app..."
cd "$PROJECT_ROOT/gui/ConvergioMesh"
swift build -c release
SWIFT_BIN="$PROJECT_ROOT/gui/ConvergioMesh/.build/release/ConvergioMesh"
[ -f "$SWIFT_BIN" ] || { echo "ERROR: Swift binary not found"; exit 1; }

# 3. Create .app bundle manually (since SPM doesn't create .app)
echo "[3/6] Creating app bundle..."
mkdir -p "$BUILD_DIR"
APP_DIR="$BUILD_DIR/$APP_NAME.app/Contents/MacOS"
RESOURCES_DIR="$BUILD_DIR/$APP_NAME.app/Contents/Resources"
mkdir -p "$APP_DIR" "$RESOURCES_DIR"

# Copy binaries
cp "$SWIFT_BIN" "$APP_DIR/$APP_NAME"
cp "$CLI_BIN" "$APP_DIR/convergiomesh-cli"

# Copy app icon
ICON_SRC="$PROJECT_ROOT/resources/AppIcon.icns"
if [[ -f "$ICON_SRC" ]]; then
    cp "$ICON_SRC" "$RESOURCES_DIR/AppIcon.icns"
    echo "  App icon copied"
fi

# Create Info.plist
cat > "$BUILD_DIR/$APP_NAME.app/Contents/Info.plist" << 'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key><string>ConvergioMesh</string>
    <key>CFBundleIdentifier</key><string>io.convergio.mesh</string>
    <key>CFBundleName</key><string>ConvergioMesh</string>
    <key>CFBundleVersion</key><string>0.1.0</string>
    <key>CFBundleShortVersionString</key><string>0.1.0</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>LSMinimumSystemVersion</key><string>14.0</string>
    <key>CFBundleIconFile</key><string>AppIcon</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>LSApplicationCategoryType</key><string>public.app-category.developer-tools</string>
</dict>
</plist>
PLIST

# 4. Ad-hoc code sign
echo "[4/6] Code signing..."
codesign --deep --force -s - "$BUILD_DIR/$APP_NAME.app"

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
