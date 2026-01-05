# CI/CD Pipeline Configuration

This document describes the complete CI/CD setup for Soul Player across all platforms.

---

## Overview

Soul Player uses **GitHub Actions** for CI/CD with separate workflows for:

1. **Libraries** - Test all Rust libraries
2. **Frontend** - Test shared React components
3. **Desktop** - Build for Windows, macOS, Linux
4. **Mobile (iOS)** - Build and test iOS app
5. **Mobile (Android)** - Build and test Android app
6. **Server** - Build server binary and Docker image
7. **Release** - Automated releases and distribution

---

## Workflow Files

All workflows are in `.github/workflows/`.

### 1. Libraries CI (`ci-libraries.yml`)

**Triggers**: Push/PR to `libraries/**`, `Cargo.toml`

```yaml
name: Libraries CI

on:
  push:
    branches: [main, develop]
    paths:
      - 'libraries/**'
      - 'Cargo.toml'
      - 'Cargo.lock'
  pull_request:
    paths:
      - 'libraries/**'
      - 'Cargo.toml'

env:
  RUST_BACKTRACE: 1
  CARGO_TERM_COLOR: always

jobs:
  test:
    name: Test Libraries
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, nightly]
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
          components: rustfmt, clippy

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Run tests
        run: |
          cd libraries
          cargo test --all --all-features

      - name: Run clippy
        run: |
          cd libraries
          cargo clippy --all-targets --all-features -- -D warnings

      - name: Check formatting
        run: |
          cd libraries
          cargo fmt --all -- --check

  coverage:
    name: Code Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install cargo-tarpaulin
        run: cargo install cargo-tarpaulin

      - name: Generate coverage
        run: |
          cd libraries
          cargo tarpaulin --all --all-features --out xml --timeout 300

      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          files: ./libraries/cobertura.xml
          flags: libraries

  audit:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Run audit
        run: cargo audit
```

---

### 2. Frontend CI (`ci-frontend.yml`)

**Triggers**: Push/PR to `applications/shared/**`

```yaml
name: Frontend CI

on:
  push:
    branches: [main, develop]
    paths:
      - 'applications/shared/**'
      - 'applications/desktop/src/**'
      - 'applications/mobile/src/**'
  pull_request:
    paths:
      - 'applications/shared/**'

jobs:
  test:
    name: Test Shared Components
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: applications/shared/package-lock.json

      - name: Install dependencies
        run: |
          cd applications/shared
          npm ci

      - name: Run linter
        run: |
          cd applications/shared
          npm run lint

      - name: Type check
        run: |
          cd applications/shared
          npm run type-check

      - name: Run tests
        run: |
          cd applications/shared
          npm run test

      - name: Coverage
        run: |
          cd applications/shared
          npm run test:coverage

      - name: Upload coverage
        uses: codecov/codecov-action@v4
        with:
          files: ./applications/shared/coverage/coverage-final.json
          flags: frontend
```

---

### 3. Desktop CI (`ci-desktop.yml`)

**Triggers**: Push/PR to `applications/desktop/**`

**Build Matrix**: Windows (x64, ARM64), macOS (Intel, Apple Silicon), Linux (x64, ARM64)

```yaml
name: Desktop CI

on:
  push:
    branches: [main, develop]
    paths:
      - 'applications/desktop/**'
      - 'applications/shared/**'
      - 'libraries/**'
  pull_request:
    paths:
      - 'applications/desktop/**'

env:
  TAURI_PRIVATE_KEY: ${{ secrets.TAURI_PRIVATE_KEY }}
  TAURI_KEY_PASSWORD: ${{ secrets.TAURI_KEY_PASSWORD }}

jobs:
  build:
    name: Build Desktop App
    strategy:
      fail-fast: false
      matrix:
        include:
          # Windows
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            arch: x64
          - os: windows-latest
            target: aarch64-pc-windows-msvc
            arch: arm64

          # macOS
          - os: macos-latest
            target: x86_64-apple-darwin
            arch: x64
          - os: macos-latest
            target: aarch64-apple-darwin
            arch: arm64

          # Linux
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            arch: x64
          - os: ubuntu-latest
            target: aarch64-unknown-linux-gnu
            arch: arm64

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: ${{ matrix.target }}

      - name: Install dependencies (Ubuntu)
        if: matrix.os == 'ubuntu-latest'
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libappindicator3-dev \
            librsvg2-dev \
            patchelf \
            libssl-dev \
            pkg-config

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'
          cache: 'npm'
          cache-dependency-path: applications/desktop/package-lock.json

      - name: Install frontend dependencies
        run: |
          cd applications/desktop
          npm ci

      - name: Build Tauri app
        run: |
          cd applications/desktop
          npm run tauri build -- --target ${{ matrix.target }}

      - name: Upload artifacts
        uses: actions/upload-artifact@v4
        with:
          name: soul-player-${{ matrix.os }}-${{ matrix.arch }}
          path: |
            applications/desktop/src-tauri/target/${{ matrix.target }}/release/bundle/

  test:
    name: Test Desktop App
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y \
            libwebkit2gtk-4.1-dev \
            libappindicator3-dev

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: |
          cd applications/desktop
          npm ci

      - name: Run Rust tests
        run: |
          cd applications/desktop/src-tauri
          cargo test

      - name: Run E2E tests
        run: |
          cd applications/desktop
          npm run test:e2e
```

---

### 4. Mobile iOS CI (`ci-mobile-ios.yml`)

**Triggers**: Push/PR to `applications/mobile/**`

**Requirements**: macOS runner, Xcode, Apple certificates

```yaml
name: Mobile iOS CI

on:
  push:
    branches: [main, develop]
    paths:
      - 'applications/mobile/**'
      - 'libraries/**'
  pull_request:
    paths:
      - 'applications/mobile/**'

jobs:
  build-ios:
    name: Build iOS App
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: aarch64-apple-ios, x86_64-apple-ios

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: |
          cd applications/mobile
          npm ci

      - name: Install CocoaPods
        run: sudo gem install cocoapods

      - name: Install iOS dependencies
        run: |
          cd applications/mobile/src-tauri/gen/apple
          pod install

      - name: Build iOS (Simulator)
        run: |
          cd applications/mobile
          npm run tauri ios build -- --target x86_64-apple-ios

      - name: Build iOS (Device)
        run: |
          cd applications/mobile
          npm run tauri ios build -- --target aarch64-apple-ios

      - name: Run tests
        run: |
          cd applications/mobile
          xcodebuild test \
            -workspace src-tauri/gen/apple/soul-mobile.xcworkspace \
            -scheme soul-mobile \
            -destination 'platform=iOS Simulator,name=iPhone 15'

  testflight:
    name: Deploy to TestFlight
    runs-on: macos-latest
    if: github.ref == 'refs/heads/main'
    needs: build-ios
    steps:
      - uses: actions/checkout@v4

      - name: Import certificates
        env:
          APPLE_CERTIFICATE_BASE64: ${{ secrets.APPLE_CERTIFICATE_BASE64 }}
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
        run: |
          # Import signing certificates
          echo $APPLE_CERTIFICATE_BASE64 | base64 --decode > certificate.p12
          security create-keychain -p "" build.keychain
          security import certificate.p12 -k build.keychain -P $APPLE_CERTIFICATE_PASSWORD
          security list-keychains -s build.keychain
          security default-keychain -s build.keychain
          security unlock-keychain -p "" build.keychain

      - name: Build and upload to TestFlight
        env:
          APP_STORE_CONNECT_API_KEY: ${{ secrets.APP_STORE_CONNECT_API_KEY }}
        run: |
          cd applications/mobile
          npm run tauri ios build -- --release
          xcrun altool --upload-app \
            --file src-tauri/target/aarch64-apple-ios/release/bundle/ios/soul-mobile.ipa \
            --apiKey $APP_STORE_CONNECT_API_KEY
```

---

### 5. Mobile Android CI (`ci-mobile-android.yml`)

**Triggers**: Push/PR to `applications/mobile/**`

```yaml
name: Mobile Android CI

on:
  push:
    branches: [main, develop]
    paths:
      - 'applications/mobile/**'
      - 'libraries/**'
  pull_request:
    paths:
      - 'applications/mobile/**'

jobs:
  build-android:
    name: Build Android App
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Setup Java
        uses: actions/setup-java@v4
        with:
          distribution: 'temurin'
          java-version: '17'

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: |
            aarch64-linux-android
            armv7-linux-androideabi
            x86_64-linux-android

      - name: Setup Android SDK
        uses: android-actions/setup-android@v3

      - name: Install NDK
        run: |
          sdkmanager --install "ndk;26.1.10909125"
          echo "ANDROID_NDK_HOME=$ANDROID_SDK_ROOT/ndk/26.1.10909125" >> $GITHUB_ENV

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20'

      - name: Install dependencies
        run: |
          cd applications/mobile
          npm ci

      - name: Build Android (Debug)
        run: |
          cd applications/mobile
          npm run tauri android build -- --debug

      - name: Build Android (Release)
        if: github.ref == 'refs/heads/main'
        env:
          ANDROID_KEYSTORE_BASE64: ${{ secrets.ANDROID_KEYSTORE_BASE64 }}
          ANDROID_KEYSTORE_PASSWORD: ${{ secrets.ANDROID_KEYSTORE_PASSWORD }}
        run: |
          # Decode keystore
          echo $ANDROID_KEYSTORE_BASE64 | base64 --decode > keystore.jks

          cd applications/mobile
          npm run tauri android build -- --release

      - name: Run Android tests
        run: |
          cd applications/mobile/src-tauri/gen/android
          ./gradlew test

      - name: Upload APK
        uses: actions/upload-artifact@v4
        with:
          name: soul-player-android
          path: |
            applications/mobile/src-tauri/gen/android/app/build/outputs/apk/

  play-store:
    name: Deploy to Play Store
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    needs: build-android
    steps:
      - uses: actions/checkout@v4

      - name: Upload to Play Store (Beta)
        env:
          PLAY_STORE_CREDENTIALS: ${{ secrets.PLAY_STORE_CREDENTIALS }}
        run: |
          # Use fastlane or Google Play Developer API
          # Upload AAB to internal testing track
          echo "Deploying to Play Store beta track"
```

---

### 6. Server CI (`ci-server.yml`)

**Triggers**: Push/PR to `applications/server/**`

```yaml
name: Server CI

on:
  push:
    branches: [main, develop]
    paths:
      - 'applications/server/**'
      - 'libraries/**'
  pull_request:
    paths:
      - 'applications/server/**'

jobs:
  test:
    name: Test Server
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
        ports:
          - 5432:5432

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Run tests
        run: |
          cd applications/server
          cargo test --all-features

  docker:
    name: Build Docker Image
    runs-on: ubuntu-latest
    needs: test
    steps:
      - uses: actions/checkout@v4

      - name: Set up Docker Buildx
        uses: docker/setup-buildx-action@v3

      - name: Login to Docker Hub
        if: github.ref == 'refs/heads/main'
        uses: docker/login-action@v3
        with:
          username: ${{ secrets.DOCKER_USERNAME }}
          password: ${{ secrets.DOCKER_PASSWORD }}

      - name: Build and push
        uses: docker/build-push-action@v5
        with:
          context: ./applications/server
          platforms: linux/amd64,linux/arm64
          push: ${{ github.ref == 'refs/heads/main' }}
          tags: |
            soulplayer/server:latest
            soulplayer/server:${{ github.sha }}
          cache-from: type=gha
          cache-to: type=gha,mode=max
```

---

### 7. Release Workflow (`release.yml`)

**Triggers**: Tag push (`v*.*.*`)

```yaml
name: Release

on:
  push:
    tags:
      - 'v*.*.*'

jobs:
  create-release:
    name: Create GitHub Release
    runs-on: ubuntu-latest
    outputs:
      upload_url: ${{ steps.create_release.outputs.upload_url }}
    steps:
      - uses: actions/checkout@v4

      - name: Create Release
        id: create_release
        uses: actions/create-release@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tag_name: ${{ github.ref }}
          release_name: Release ${{ github.ref }}
          draft: false
          prerelease: false

  desktop-release:
    name: Desktop Release
    needs: create-release
    strategy:
      matrix:
        include:
          - os: windows-latest
            target: x86_64-pc-windows-msvc
            ext: .msi
          - os: macos-latest
            target: aarch64-apple-darwin
            ext: .dmg
          - os: ubuntu-latest
            target: x86_64-unknown-linux-gnu
            ext: .AppImage

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - name: Build desktop app
        run: |
          cd applications/desktop
          npm ci
          npm run tauri build -- --target ${{ matrix.target }}

      - name: Upload release asset
        uses: actions/upload-release-asset@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          upload_url: ${{ needs.create-release.outputs.upload_url }}
          asset_path: ./applications/desktop/src-tauri/target/${{ matrix.target }}/release/bundle/*${{ matrix.ext }}
          asset_name: soul-player-${{ matrix.os }}${{ matrix.ext }}
          asset_content_type: application/octet-stream

  mobile-release:
    name: Mobile Release
    needs: create-release
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4

      # iOS App Store submission
      - name: Build and submit iOS
        run: |
          cd applications/mobile
          npm run tauri ios build -- --release
          # Submit to App Store Connect

      # Android Play Store submission
      - name: Build and submit Android
        run: |
          cd applications/mobile
          npm run tauri android build -- --release
          # Submit to Play Store
```

---

## Secrets Configuration

### GitHub Secrets Required

**Desktop**:
- `TAURI_PRIVATE_KEY` - Tauri updater signing key
- `TAURI_KEY_PASSWORD` - Signing key password

**iOS**:
- `APPLE_CERTIFICATE_BASE64` - Apple distribution certificate
- `APPLE_CERTIFICATE_PASSWORD` - Certificate password
- `APP_STORE_CONNECT_API_KEY` - App Store Connect API key

**Android**:
- `ANDROID_KEYSTORE_BASE64` - Android signing keystore
- `ANDROID_KEYSTORE_PASSWORD` - Keystore password
- `PLAY_STORE_CREDENTIALS` - Play Store service account JSON

**Docker**:
- `DOCKER_USERNAME` - Docker Hub username
- `DOCKER_PASSWORD` - Docker Hub password/token

---

## Build Matrix Summary

| Platform | OS | Architectures | Artifacts |
|----------|----|--------------|-----------|
| Windows | windows-latest | x64, ARM64 | .msi, .exe |
| macOS | macos-latest | x64, ARM64 (Universal) | .dmg, .app |
| Linux | ubuntu-latest | x64, ARM64 | .AppImage, .deb |
| iOS | macos-latest | ARM64 (device), x64 (simulator) | .ipa |
| Android | ubuntu-latest | ARM64, ARMv7, x64 | .apk, .aab |
| Server | ubuntu-latest | x64, ARM64 | Docker image |

---

## Performance Optimizations

### Caching Strategy
- Cargo dependencies cached by lock file hash
- npm dependencies cached by lock file hash
- Rust build artifacts cached
- Docker layer caching with GitHub Actions cache

### Parallel Builds
- Matrix builds run in parallel
- Libraries tested across OS/Rust versions simultaneously
- Desktop builds for all platforms at once

### Incremental Compilation
- Rust incremental compilation enabled in dev
- Disabled for release builds (smaller binaries)

---

## Monitoring & Notifications

### Status Badges (README.md)
```markdown
![Libraries CI](https://github.com/yourusername/soul-player/workflows/Libraries%20CI/badge.svg)
![Desktop CI](https://github.com/yourusername/soul-player/workflows/Desktop%20CI/badge.svg)
![Mobile iOS](https://github.com/yourusername/soul-player/workflows/Mobile%20iOS%20CI/badge.svg)
![Coverage](https://codecov.io/gh/yourusername/soul-player/branch/main/graph/badge.svg)
```

### Slack/Discord Notifications
- Build failures notify team channel
- Release notifications
- Security audit alerts

---

## Local Development Scripts

### `scripts/ci/install-deps.sh`
```bash
#!/bin/bash
# Install all CI dependencies locally for testing

if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    sudo apt-get update
    sudo apt-get install -y \
        libwebkit2gtk-4.1-dev \
        libappindicator3-dev \
        librsvg2-dev \
        patchelf
elif [[ "$OSTYPE" == "darwin"* ]]; then
    brew install cocoapods
fi

# Install Rust targets
rustup target add \
    x86_64-pc-windows-msvc \
    aarch64-apple-darwin \
    aarch64-apple-ios \
    aarch64-linux-android

# Install tools
cargo install cargo-tarpaulin cargo-audit
```

### `scripts/test-all.sh`
```bash
#!/bin/bash
# Run all tests locally (mimics CI)

set -e

echo "Testing libraries..."
cd libraries && cargo test --all && cd ..

echo "Testing shared frontend..."
cd applications/shared && npm test && cd ../..

echo "Testing desktop..."
cd applications/desktop && cargo test && npm test && cd ../..

echo "Testing mobile..."
cd applications/mobile && cargo test && cd ../..

echo "All tests passed!"
```

---

## Troubleshooting CI Failures

### Common Issues

**1. Rust compilation errors on specific OS**
- Check target is installed: `rustup target list --installed`
- Verify system dependencies (Ubuntu: webkit, macOS: Xcode)

**2. Tauri build fails**
- Ensure Node.js version matches (20+)
- Clear npm cache: `npm cache clean --force`
- Check Tauri CLI version

**3. Mobile builds fail**
- iOS: Check Xcode version, certificates
- Android: Verify NDK version, Java 17

**4. Tests timeout**
- Increase timeout in workflow (default: 30min)
- Check for hanging processes
- Review test logs

---

## Next Steps

1. **Set up GitHub secrets** for signing and distribution
2. **Configure Codecov** for coverage reports
3. **Set up Dependabot** for automated dependency updates
4. **Configure branch protection** (require CI to pass)
5. **Add performance benchmarks** to CI

---

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Tauri CI/CD Guide](https://v2.tauri.app/develop/tests/webdriver/ci/)
- [cargo-tarpaulin](https://github.com/xd009642/tarpaulin)
- [WebdriverIO](https://webdriver.io/)
