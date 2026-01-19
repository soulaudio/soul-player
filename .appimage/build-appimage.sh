#!/usr/bin/env bash
# Build AppImage for Soul Player
# This script creates a portable AppImage that works on all Linux distributions

set -e

VERSION="${1:-0.1.1}"
ARCH="x86_64"
APP_NAME="Soul Player"
APP_ID="soul-player"
BINARY_NAME="soul-player"

echo "Building AppImage for Soul Player v${VERSION}..."

# Install dependencies if needed
if ! command -v linuxdeploy-x86_64.AppImage &> /dev/null; then
    echo "Downloading linuxdeploy..."
    wget -q https://github.com/linuxdeploy/linuxdeploy/releases/download/continuous/linuxdeploy-x86_64.AppImage
    chmod +x linuxdeploy-x86_64.AppImage
fi

# Prepare AppDir structure
echo "Creating AppDir structure..."
rm -rf AppDir
mkdir -p AppDir/usr/bin
mkdir -p AppDir/usr/share/applications
mkdir -p AppDir/usr/share/icons/hicolor/{32x32,128x128,256x256}/apps

# Copy binary
echo "Copying binary..."
if [ -f "target/release/${BINARY_NAME}" ]; then
    cp "target/release/${BINARY_NAME}" AppDir/usr/bin/
else
    echo "Error: Binary not found at target/release/${BINARY_NAME}"
    echo "Please build the project first with: cargo build --release"
    exit 1
fi

# Create desktop file
echo "Creating desktop file..."
cat > AppDir/usr/share/applications/${APP_ID}.desktop << EOF
[Desktop Entry]
Name=${APP_NAME}
Exec=${BINARY_NAME}
Icon=${APP_ID}
Type=Application
Categories=AudioVideo;Audio;Player;
Comment=Modern, privacy-first music player for local audio files
Terminal=false
StartupNotify=true
EOF

# Copy icons
echo "Copying icons..."
for size in 32 128 256; do
    ICON_PATH="applications/desktop/src-tauri/icons/${size}x${size}.png"
    if [ -f "$ICON_PATH" ]; then
        cp "$ICON_PATH" "AppDir/usr/share/icons/hicolor/${size}x${size}/apps/${APP_ID}.png"
    else
        echo "Warning: Icon not found at $ICON_PATH"
    fi
done

# Set main icon
if [ -f "applications/desktop/src-tauri/icons/128x128.png" ]; then
    cp "applications/desktop/src-tauri/icons/128x128.png" "AppDir/${APP_ID}.png"
fi

# Build AppImage
echo "Building AppImage..."
./linuxdeploy-x86_64.AppImage \
    --appdir AppDir \
    --executable "AppDir/usr/bin/${BINARY_NAME}" \
    --desktop-file "AppDir/usr/share/applications/${APP_ID}.desktop" \
    --icon-file "AppDir/${APP_ID}.png" \
    --output appimage

# Rename to consistent naming
if [ -f "${APP_NAME}-${ARCH}.AppImage" ]; then
    mv "${APP_NAME}-${ARCH}.AppImage" "${APP_ID}_${VERSION}_${ARCH}.AppImage"
    echo "✅ AppImage created: ${APP_ID}_${VERSION}_${ARCH}.AppImage"

    # Calculate SHA256
    sha256sum "${APP_ID}_${VERSION}_${ARCH}.AppImage" > "${APP_ID}_${VERSION}_${ARCH}.AppImage.sha256"
    echo "✅ Checksum created: ${APP_ID}_${VERSION}_${ARCH}.AppImage.sha256"
else
    echo "❌ Error: AppImage was not created"
    exit 1
fi

echo "Done!"
