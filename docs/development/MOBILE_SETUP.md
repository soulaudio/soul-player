# Mobile Development Setup

Complete guide for setting up iOS and Android development for Soul Player.

---

## Prerequisites

### Required for All Platforms
- **Rust**: 1.75+ (install via `rustup`)
- **Node.js**: 20+ (install via `nvm` or official installer)
- **Tauri CLI**: `npm install -g @tauri-apps/cli@next`

### iOS Development (macOS only)
- **Xcode**: 15.0+ (from App Store)
- **Xcode Command Line Tools**: `xcode-select --install`
- **CocoaPods**: `sudo gem install cocoapods`
- **Apple Developer Account**: Required for device testing and distribution

### Android Development (All platforms)
- **Android Studio**: Latest stable version
- **Android SDK**: API Level 33+
- **Android NDK**: Version 26.1.10909125
- **Java JDK**: 17+ (Temurin recommended)

---

## iOS Setup (macOS)

### 1. Install Dependencies

```bash
# Install Xcode Command Line Tools
xcode-select --install

# Install CocoaPods
sudo gem install cocoapods

# Install Rust iOS targets
rustup target add aarch64-apple-ios       # Device
rustup target add x86_64-apple-ios        # Simulator (Intel)
rustup target add aarch64-apple-ios-sim   # Simulator (Apple Silicon)
```

### 2. Configure Xcode

```bash
# Accept Xcode license
sudo xcodebuild -license accept

# Verify installation
xcodebuild -version
# Should output: Xcode 15.x
```

### 3. Initialize Mobile Project

```bash
cd applications/mobile

# Install npm dependencies
npm install

# Initialize Tauri mobile
npm run tauri ios init
```

This creates:
- `src-tauri/gen/apple/` - Xcode project
- `src-tauri/gen/apple/soul-mobile.xcodeproj` - Xcode workspace
- `src-tauri/gen/apple/Sources/` - Swift bridge code

### 4. Configure Signing

Open Xcode project:
```bash
cd src-tauri/gen/apple
open soul-mobile.xcodeproj
```

In Xcode:
1. Select project in navigator
2. Go to "Signing & Capabilities"
3. Select your Team (Apple Developer account)
4. Set Bundle Identifier: `com.soulplayer.mobile`

### 5. Run on Simulator

```bash
cd applications/mobile

# List available simulators
npm run tauri ios dev -- --target simulator --list

# Run on iPhone 15 simulator
npm run tauri ios dev -- --target simulator --device "iPhone 15"
```

### 6. Run on Device

```bash
# Connect iOS device via USB

# List connected devices
npm run tauri ios dev -- --target device --list

# Run on connected device
npm run tauri ios dev -- --target device
```

**Note**: Device must be registered in your Apple Developer account.

### 7. Build for Release

```bash
# Build IPA for distribution
npm run tauri ios build -- --release

# Output: src-tauri/target/aarch64-apple-ios/release/bundle/ios/soul-mobile.ipa
```

---

## Android Setup

### 1. Install Android Studio

Download from: https://developer.android.com/studio

During installation, select:
- Android SDK
- Android SDK Platform (API 33+)
- Android Virtual Device

### 2. Install Android NDK

```bash
# Open Android Studio
# Go to: Tools > SDK Manager > SDK Tools
# Check "NDK (Side by side)" - version 26.1.10909125
# Click "Apply" to install

# Set environment variable
export ANDROID_NDK_HOME="$HOME/Library/Android/sdk/ndk/26.1.10909125"  # macOS/Linux
# or
set ANDROID_NDK_HOME=%USERPROFILE%\AppData\Local\Android\Sdk\ndk\26.1.10909125  # Windows
```

Add to `.bashrc` or `.zshrc`:
```bash
export ANDROID_HOME="$HOME/Library/Android/sdk"  # macOS
export ANDROID_NDK_HOME="$ANDROID_HOME/ndk/26.1.10909125"
export PATH="$PATH:$ANDROID_HOME/platform-tools:$ANDROID_HOME/tools"
```

### 3. Install Java JDK 17

```bash
# macOS (via Homebrew)
brew install openjdk@17

# Ubuntu
sudo apt install openjdk-17-jdk

# Windows
# Download from: https://adoptium.net/
```

Verify:
```bash
java -version
# Should output: openjdk version "17.x.x"
```

### 4. Install Rust Android Targets

```bash
rustup target add \
    aarch64-linux-android \
    armv7-linux-androideabi \
    x86_64-linux-android \
    i686-linux-android
```

### 5. Initialize Mobile Project

```bash
cd applications/mobile

# Install npm dependencies
npm install

# Initialize Tauri Android
npm run tauri android init
```

This creates:
- `src-tauri/gen/android/` - Android Studio project
- `src-tauri/gen/android/app/` - Main app module
- `src-tauri/gen/android/app/src/main/java/` - Kotlin bridge code

### 6. Configure Signing (Development)

Android Studio generates a debug keystore automatically.

For production, create release keystore:
```bash
keytool -genkey -v \
  -keystore soul-player-release.keystore \
  -alias soul-player \
  -keyalg RSA \
  -keysize 2048 \
  -validity 10000
```

Update `src-tauri/gen/android/app/build.gradle`:
```groovy
android {
    signingConfigs {
        release {
            storeFile file("../../../soul-player-release.keystore")
            storePassword System.getenv("KEYSTORE_PASSWORD")
            keyAlias "soul-player"
            keyPassword System.getenv("KEY_PASSWORD")
        }
    }
}
```

### 7. Run on Emulator

```bash
cd applications/mobile

# List available emulators
npm run tauri android dev -- --emulator --list

# Create emulator (if needed)
avdmanager create avd \
  -n Pixel_7 \
  -k "system-images;android-33;google_apis;x86_64"

# Run on emulator
npm run tauri android dev -- --emulator Pixel_7
```

### 8. Run on Device

```bash
# Enable USB debugging on Android device:
# Settings > Developer Options > USB Debugging

# Connect device via USB

# List connected devices
adb devices

# Run on connected device
npm run tauri android dev -- --device
```

### 9. Build for Release

```bash
# Set keystore credentials
export KEYSTORE_PASSWORD="your-password"
export KEY_PASSWORD="your-key-password"

# Build APK
npm run tauri android build -- --release --apk

# Build AAB (for Play Store)
npm run tauri android build -- --release --aab

# Output:
# APK: src-tauri/gen/android/app/build/outputs/apk/release/app-release.apk
# AAB: src-tauri/gen/android/app/build/outputs/bundle/release/app-release.aab
```

---

## Platform-Specific Bridge Code

### iOS Audio Bridge (Swift)

**Location**: `applications/mobile/src-tauri/gen/apple/Sources/AudioBridge.swift`

```swift
import AVFoundation
import Tauri

@_cdecl("init_plugin_audio_bridge")
func initPlugin() -> Plugin {
    return AudioBridge()
}

class AudioBridge: Plugin {
    private var audioEngine: AVAudioEngine?
    private var playerNode: AVAudioPlayerNode?

    @objc func setupAudioEngine(_ invoke: Invoke) {
        let args = invoke.parseArgs([String: Any].self)
        guard let sampleRate = args?["sampleRate"] as? Double,
              let channels = args?["channels"] as? UInt32 else {
            invoke.reject("Invalid arguments")
            return
        }

        audioEngine = AVAudioEngine()
        playerNode = AVAudioPlayerNode()

        guard let engine = audioEngine, let player = playerNode else {
            invoke.reject("Failed to initialize audio engine")
            return
        }

        engine.attach(player)

        let format = AVAudioFormat(
            standardFormatWithSampleRate: sampleRate,
            channels: channels
        )!

        engine.connect(player, to: engine.mainMixerNode, format: format)

        do {
            try engine.start()
            invoke.resolve()
        } catch {
            invoke.reject("Failed to start audio engine: \(error)")
        }
    }

    @objc func playBuffer(_ invoke: Invoke) {
        // Implementation for playing audio buffers
        invoke.resolve()
    }
}
```

**Register in** `applications/mobile/src-tauri/src/lib.rs`:

```rust
#[cfg(target_os = "ios")]
#[link(name = "AudioBridge", kind = "static")]
extern "C" {
    fn init_plugin_audio_bridge() -> *const ();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        #[cfg(target_os = "ios")]
        .plugin(unsafe {
            std::mem::transmute::<*const (), Box<dyn tauri::plugin::Plugin>>(
                init_plugin_audio_bridge()
            )
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Android Audio Bridge (Kotlin)

**Location**: `applications/mobile/src-tauri/gen/android/app/src/main/java/com/soulplayer/mobile/AudioBridge.kt`

```kotlin
package com.soulplayer.mobile

import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioTrack
import app.tauri.annotation.Command
import app.tauri.annotation.TauriPlugin
import app.tauri.plugin.Invoke
import app.tauri.plugin.Plugin

@TauriPlugin
class AudioBridge(private val activity: Activity) : Plugin(activity) {
    private var audioTrack: AudioTrack? = null

    @Command
    fun setupAudioEngine(invoke: Invoke) {
        val sampleRate = invoke.getInt("sampleRate") ?: 44100
        val channels = invoke.getInt("channels") ?: 2

        val channelConfig = if (channels == 2) {
            AudioFormat.CHANNEL_OUT_STEREO
        } else {
            AudioFormat.CHANNEL_OUT_MONO
        }

        val bufferSize = AudioTrack.getMinBufferSize(
            sampleRate,
            channelConfig,
            AudioFormat.ENCODING_PCM_FLOAT
        )

        audioTrack = AudioTrack.Builder()
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_MEDIA)
                    .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                    .build()
            )
            .setAudioFormat(
                AudioFormat.Builder()
                    .setSampleRate(sampleRate)
                    .setChannelMask(channelConfig)
                    .setEncoding(AudioFormat.ENCODING_PCM_FLOAT)
                    .build()
            )
            .setBufferSizeInBytes(bufferSize)
            .setTransferMode(AudioTrack.MODE_STREAM)
            .build()

        audioTrack?.play()
        invoke.resolve()
    }

    @Command
    fun playBuffer(invoke: Invoke) {
        val samples = invoke.getFloatArray("samples") ?: return
        audioTrack?.write(samples, 0, samples.size, AudioTrack.WRITE_NON_BLOCKING)
        invoke.resolve()
    }

    override fun destroy() {
        audioTrack?.stop()
        audioTrack?.release()
        super.destroy()
    }
}
```

**Register in** `src-tauri/gen/android/app/src/main/java/com/soulplayer/mobile/MainActivity.kt`:

```kotlin
import com.soulplayer.mobile.AudioBridge

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Register plugin
        registerPlugin(AudioBridge::class.java)
    }
}
```

---

## Development Workflow

### Hot Reload

Both iOS and Android support Hot Module Replacement (HMR):

```bash
# iOS
npm run tauri ios dev

# Android
npm run tauri android dev

# Frontend changes reload automatically
# Rust changes require rebuild
```

### Debugging

**iOS (Xcode)**:
```bash
# Open in Xcode
cd src-tauri/gen/apple
open soul-mobile.xcodeproj

# Set breakpoints in Swift code
# Run from Xcode (Cmd+R)
# View logs in Console
```

**Android (Android Studio)**:
```bash
# Open in Android Studio
cd src-tauri/gen/android
studio .

# Set breakpoints in Kotlin code
# Run from Android Studio (Shift+F10)
# View logs in Logcat
```

**React DevTools**:
```bash
# Enable remote debugging
npm run tauri ios dev -- --remote-debugging

# Open in browser: chrome://inspect
```

---

## Testing on Mobile

### Unit Tests

```bash
# Frontend tests (Vitest)
cd applications/mobile
npm run test

# Rust tests
cd src-tauri
cargo test
```

### Integration Tests (iOS)

```bash
cd src-tauri/gen/apple

# Run XCTest suite
xcodebuild test \
  -workspace soul-mobile.xcworkspace \
  -scheme soul-mobile \
  -destination 'platform=iOS Simulator,name=iPhone 15'
```

### Integration Tests (Android)

```bash
cd src-tauri/gen/android

# Run instrumented tests
./gradlew connectedAndroidTest
```

---

## Common Issues

### iOS

**Issue**: "Developer Mode disabled"
**Solution**: iOS 16+ requires enabling Developer Mode in Settings

**Issue**: "Untrusted Developer"
**Solution**: Settings > General > VPN & Device Management > Trust

**Issue**: CocoaPods not found
**Solution**: `sudo gem install cocoapods`

### Android

**Issue**: "ANDROID_HOME not set"
**Solution**: Set environment variable to SDK path

**Issue**: "NDK not found"
**Solution**: Install via Android Studio SDK Manager

**Issue**: "Gradle build failed"
**Solution**: Check Java version (must be 17)

---

## Distribution

### iOS (App Store)

1. Archive in Xcode (Product > Archive)
2. Validate app
3. Upload to App Store Connect
4. Submit for review

Or use Fastlane:
```bash
cd src-tauri/gen/apple
fastlane ios release
```

### Android (Play Store)

1. Build AAB: `npm run tauri android build -- --aab`
2. Upload to Play Console
3. Create release
4. Submit for review

Or use Fastlane:
```bash
cd src-tauri/gen/android
fastlane android release
```

---

## Resources

- [Tauri Mobile Guide](https://v2.tauri.app/start/prerequisites/)
- [iOS Development](https://developer.apple.com/ios/)
- [Android Development](https://developer.android.com/)
- [Xcode Download](https://developer.apple.com/xcode/)
- [Android Studio Download](https://developer.android.com/studio)
