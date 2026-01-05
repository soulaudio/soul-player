# Tauri Icons

This directory should contain the application icons for Soul Player Desktop.

## Required Icons

Tauri requires the following icon files:

- `32x32.png` - 32x32 PNG icon
- `128x128.png` - 128x128 PNG icon
- `128x128@2x.png` - 256x256 PNG icon (2x retina)
- `icon.icns` - macOS icon file (contains multiple sizes)
- `icon.ico` - Windows icon file (contains multiple sizes)

## Generating Icons

You can use the Tauri icon generator:

```bash
# From repository root
cd applications/desktop
yarn tauri icon /path/to/your/icon.png
```

The icon generator will create all required sizes and formats automatically.

## Temporary Solution

Until proper icons are created, Tauri will use default placeholder icons during development.

For production builds, you MUST replace these with proper icons.
