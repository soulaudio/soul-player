# Soul Player Mobile

iOS and Android music player.

## Setup

```bash
# From repository root
yarn workspace soul-player-mobile tauri:ios init
yarn workspace soul-player-mobile tauri:ios dev

# Or from applications/mobile/
cd applications/mobile
yarn install

# iOS
yarn tauri:ios init
yarn tauri:ios dev

# Android
yarn tauri:android init
yarn tauri:android dev
```

## Build

```bash
# From repository root
yarn build:mobile

# Or from applications/mobile/ for specific platforms
# iOS
yarn tauri:ios build

# Android
yarn tauri:android build --apk
yarn tauri:android build --aab
```

## Requirements

**iOS**: macOS, Xcode 15+, CocoaPods
**Android**: Android Studio, SDK 33+, NDK, Java 17

See `docs/development/MOBILE_SETUP.md`
