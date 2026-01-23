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

# Find the created AppImage (appimagetool replaces spaces with underscores)
# Output is "Soul_Player-x86_64.AppImage" or similar pattern
echo "Searching for created AppImage..."
ls -la *.AppImage 2>/dev/null || true

# Look for the app-specific AppImage (not linuxdeploy)
APPIMAGE_FILE=""
for file in Soul_Player-*.AppImage "Soul Player-"*.AppImage; do
    if [ -f "$file" ]; then
        APPIMAGE_FILE="$file"
        break
    fi
done

if [ -n "$APPIMAGE_FILE" ] && [ -f "$APPIMAGE_FILE" ]; then
    # Rename to consistent naming
    TARGET_NAME="${APP_ID}_${VERSION}_${ARCH}.AppImage"
    mv "$APPIMAGE_FILE" "$TARGET_NAME"
    echo "✅ AppImage created: $TARGET_NAME (from $APPIMAGE_FILE)"

    # Calculate SHA256 checksum
    sha256sum "$TARGET_NAME" > "${TARGET_NAME}.sha256"
    echo "✅ Checksum created: ${TARGET_NAME}.sha256"

    # Generate Tauri signature (.sig) if signing keys are available
    # This matches the signature format used by DEB/RPM/DMG bundles
    if [ -n "$TAURI_SIGNING_PRIVATE_KEY" ] && [ -n "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" ]; then
        echo "Generating Tauri signature for auto-updates..."

        # Install tauri-cli if not already installed
        if ! command -v tauri &> /dev/null; then
            echo "Installing tauri-cli for signature generation..."
            cargo install tauri-cli --version "^2.0.0" --locked || {
                echo "⚠️  Warning: Failed to install tauri-cli, skipping signature generation"
                echo "   AppImage will still work, but auto-updates may not verify signature"
            }
        fi

        # Generate signature using tauri signer
        # Note: cargo install tauri-cli installs "cargo-tauri" binary
        if command -v cargo-tauri &> /dev/null || command -v tauri &> /dev/null; then
            # Save private key to temp file
            TEMP_KEY_FILE=$(mktemp)
            echo "$TAURI_SIGNING_PRIVATE_KEY" > "$TEMP_KEY_FILE"

            # Sign the AppImage using cargo tauri (tauri-cli installs as cargo-tauri)
            TAURI_CMD="cargo tauri"
            if ! command -v cargo-tauri &> /dev/null && command -v tauri &> /dev/null; then
                TAURI_CMD="tauri"
            fi

            echo "Using command: $TAURI_CMD signer sign"
            $TAURI_CMD signer sign "$TARGET_NAME" \
                --private-key "$TEMP_KEY_FILE" \
                --password "$TAURI_SIGNING_PRIVATE_KEY_PASSWORD" || {
                echo "⚠️  Warning: Signature generation failed"
                echo "   AppImage will still work, but auto-updates may not verify signature"
            }

            # Clean up temp key file
            rm -f "$TEMP_KEY_FILE"

            if [ -f "${TARGET_NAME}.sig" ]; then
                echo "✅ Tauri signature created: ${TARGET_NAME}.sig"
            else
                echo "⚠️  Warning: Signature file not created"
            fi
        else
            echo "⚠️  Warning: tauri-cli not found in PATH"
        fi
    else
        echo "ℹ️  Tauri signing keys not found (TAURI_SIGNING_PRIVATE_KEY not set)"
        echo "   AppImage will work but won't have Tauri signature for auto-updates"
    fi
else
    echo "❌ Error: AppImage was not created"
    echo "Current directory contents:"
    ls -la .
    echo "Looking for patterns: Soul_Player-*.AppImage or 'Soul Player-'*.AppImage"
    exit 1
fi

echo "Done!"
