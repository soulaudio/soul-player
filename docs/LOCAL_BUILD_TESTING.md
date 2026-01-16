# Local Build Testing

This document explains how to test the release pipeline builds locally using Docker before pushing to CI.

## Overview

The local build testing system emulates the GitHub Actions CI pipeline on your local machine using Docker containers. This allows you to:

- ✅ Catch build errors before pushing to CI
- ✅ Test on Linux even if you're on Windows/macOS
- ✅ Test cross-compilation for Windows from Linux
- ✅ Save CI minutes and iteration time
- ✅ Debug build issues locally with full control

## Prerequisites

### Required

- **Docker Desktop** (Windows/macOS) or **Docker Engine** (Linux)
  - Windows: [Download Docker Desktop](https://www.docker.com/products/docker-desktop/)
  - macOS: [Download Docker Desktop](https://www.docker.com/products/docker-desktop/)
  - Linux: Install via package manager (`apt install docker.io docker-compose`)

### Recommended

- **At least 8GB RAM** available for Docker
- **20GB free disk space** for build artifacts and caches
- **Fast internet** for initial Docker image build (downloads Rust, Node.js, etc.)

## Quick Start

### Linux/macOS

```bash
# Test Linux build (check, test, and build)
./scripts/local-build-test.sh linux check

# Build Linux packages (DEB, RPM, AppImage)
./scripts/local-build-test.sh linux packages

# Test Windows cross-compilation
./scripts/local-build-test.sh windows build

# Build everything
./scripts/local-build-test.sh all packages
```

### Windows (PowerShell)

```powershell
# Test Linux build (check, test, and build)
.\scripts\local-build-test.ps1 linux check

# Build Linux packages (DEB, RPM, AppImage)
.\scripts\local-build-test.ps1 linux packages

# Test Windows cross-compilation
.\scripts\local-build-test.ps1 windows build

# Build everything
.\scripts\local-build-test.ps1 all packages
```

## Build Types

| Build Type | Description | Speed | Use Case |
|------------|-------------|-------|----------|
| `check` | Run format, clippy, and type checks | Fast (2-5 min) | Before committing |
| `test` | Run all tests (Rust + frontend) | Medium (5-10 min) | Before pushing |
| `build` | Build Rust binaries only | Medium (10-15 min) | Test compilation |
| `packages` | Build full installers/packages | Slow (15-30 min) | Before release |

## Platform Support

### Linux (Full Support ✅)

Builds on Ubuntu 22.04 container matching GitHub Actions.

**Outputs:**
- `soul-player_*.deb` - Debian/Ubuntu package
- `soul-player-*.rpm` - Fedora/RHEL package
- `soul-player-*.AppImage` - Universal Linux binary

**Location:** `applications/desktop/src-tauri/target/release/bundle/`

### Windows (Cross-Compilation 🔶)

Cross-compiles Windows binary from Linux using MinGW. **Note:** This builds the Rust binary only, not the full MSI/NSIS installers (which require Windows).

**Outputs:**
- `soul-player.exe` - Windows executable

**Location:** `applications/desktop/src-tauri/target/x86_64-pc-windows-gnu/release/`

**Why cross-compilation?**
- Windows containers are not well-supported on Docker Desktop for Mac/Linux
- Requires Windows host with Hyper-V enabled
- Cross-compilation catches most build issues without needing Windows

**Testing full Windows installers:**
- Use GitHub Actions (free for open source)
- Use a Windows VM or native Windows machine
- Use cloud CI services

### macOS (Not Supported ❌)

macOS builds cannot be easily containerized due to Apple's licensing restrictions. To test macOS builds:

1. Use a macOS machine (physical or VM)
2. Run `yarn build:desktop` directly
3. Use GitHub Actions (provides macOS runners)

## Advanced Usage

### Skip Docker Image Rebuild

If you've already built the Docker images and just want to run builds:

```bash
# Linux/macOS
SKIP_DOCKER_BUILD=1 ./scripts/local-build-test.sh linux packages

# Windows PowerShell
.\scripts\local-build-test.ps1 linux packages -SkipDockerBuild
```

### Keep Container Running (for debugging)

```bash
# Linux/macOS
KEEP_CONTAINER=1 ./scripts/local-build-test.sh linux check

# Windows PowerShell
.\scripts\local-build-test.ps1 linux check -KeepContainer
```

### Verbose Output

Show all build output in console instead of logging to file:

```bash
# Linux/macOS
VERBOSE=1 ./scripts/local-build-test.sh linux build

# Windows PowerShell
.\scripts\local-build-test.ps1 linux build -Verbose
```

### Manual Docker Usage

Run specific commands in the container:

```bash
cd docker/build-env

# Linux build environment
docker compose run --rm --profile linux linux-builder bash

# Inside container:
yarn install
cargo test --all
yarn build:desktop --bundles deb
exit

# Windows cross-compilation environment
docker compose run --rm --profile windows windows-cross-builder bash
```

## Caching

Docker volumes are used to cache dependencies and speed up subsequent builds:

- `cargo-cache` - Cargo registry cache (~2GB)
- `cargo-git` - Cargo git dependencies (~500MB)
- `yarn-cache` - Yarn/npm packages (~500MB)

**Clear caches if needed:**

```bash
cd docker/build-env
docker compose down -v
```

## Troubleshooting

### "Docker is not running"

**Solution:** Start Docker Desktop or Docker Engine service.

```bash
# Linux (systemd)
sudo systemctl start docker

# macOS/Windows
# Start Docker Desktop application
```

### "Out of disk space"

**Solution:** Clean up Docker images and volumes.

```bash
# Remove build artifacts
cd docker/build-env
docker compose down -v

# Prune unused Docker data
docker system prune -a --volumes
```

### "Permission denied" (Linux)

**Solution:** Add your user to the `docker` group.

```bash
sudo usermod -aG docker $USER
newgrp docker
```

### Build fails with "cannot find -lxyz" (Windows cross-compilation)

**Known limitation:** Some native dependencies don't cross-compile well. This is expected for Tauri's WebView dependencies. The cross-compilation environment is primarily for testing core Rust code, not full desktop app builds.

**Solution:** Use GitHub Actions for full Windows builds with MSI/NSIS installers.

### Slow builds on Windows/macOS

**Docker performance on Windows/macOS:**
- Docker Desktop runs in a VM, which is slower than native Linux
- Use WSL2 backend on Windows for better performance
- Consider using a Linux VM or cloud instance for faster builds

### "Cannot connect to Docker daemon"

**Solution:** Ensure Docker Desktop is running and properly configured.

```bash
# Check Docker status
docker ps

# Restart Docker Desktop if needed
```

## Comparison: Local vs CI

| Aspect | Local (Docker) | GitHub Actions CI |
|--------|---------------|-------------------|
| Speed | Depends on hardware | Consistent |
| Cost | Free | Free (open source) |
| Linux builds | ✅ Full support | ✅ Full support |
| Windows builds | 🔶 Binary only | ✅ Full installers |
| macOS builds | ❌ Not supported | ✅ Full support |
| Feedback time | Immediate | 10-30 min queue + build |
| Debugging | ✅ Full access | Limited logs |

## Recommended Workflow

1. **Before committing:**
   ```bash
   ./scripts/local-build-test.sh linux check
   ```

2. **Before pushing:**
   ```bash
   ./scripts/local-build-test.sh linux test
   ```

3. **Before creating release:**
   ```bash
   ./scripts/local-build-test.sh linux packages
   ```

4. **Let CI handle:**
   - Windows installers (MSI/NSIS)
   - macOS installers (DMG)
   - Full integration testing
   - Release publishing

## CI Integration

The local build scripts mirror the CI pipeline exactly:

```
Local Script                   →  CI Job
─────────────────────────────────────────────────────────
ci-build-check.sh              →  Rust checks (CI)
ci-build-test.sh               →  Test stage (CI)
ci-build-binary.sh             →  Build stage (CI)
ci-build-packages.sh           →  Release stage (CI)
```

This ensures that if your local build passes, CI will likely pass too.

## Performance Tips

1. **Use check instead of packages** for quick iteration
2. **Run SKIP_DOCKER_BUILD=1** after first build to save time
3. **Keep Docker Desktop running** to avoid VM startup time
4. **Allocate more resources** to Docker in settings (RAM/CPU)
5. **Use SSD storage** for Docker volumes

## Getting Help

If you encounter issues:

1. Check this documentation first
2. Run with `VERBOSE=1` flag for detailed logs
3. Check `build-*.log` files in project root
4. Search [GitHub Issues](https://github.com/soulaudio/soul-player/issues)
5. Ask in discussions or open a new issue

## Next Steps

- [CONTRIBUTING.md](../CONTRIBUTING.md) - General contribution guidelines
- [ARCHITECTURE.md](./ARCHITECTURE.md) - Project architecture overview
- [ROADMAP.md](../ROADMAP.md) - Future development plans
