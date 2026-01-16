# Build Environment Containers

Docker-based build environments for local testing of CI pipelines.

## Quick Start

From project root:

```bash
# Linux/macOS
./scripts/local-build-test.sh linux check

# Windows (PowerShell)
.\scripts\local-build-test.ps1 linux check
```

## Contents

- `Dockerfile.linux` - Ubuntu 22.04 build environment (matches CI)
- `Dockerfile.windows-cross` - MinGW cross-compilation for Windows
- `docker-compose.yml` - Orchestration with volume caching

## Usage

### Build Images

```bash
cd docker/build-env
docker compose build linux-builder
docker compose build windows-cross-builder
```

### Run Interactive Shell

```bash
# Linux environment
docker compose run --rm --profile linux linux-builder bash

# Windows cross-compilation environment
docker compose run --rm --profile windows windows-cross-builder bash
```

### Volume Caching

Caches are persisted in Docker volumes for faster subsequent builds:

- `cargo-cache` - Rust registry (~2GB)
- `cargo-git` - Cargo git dependencies (~500MB)
- `yarn-cache` - Node packages (~500MB)

Clear caches:
```bash
docker compose down -v
```

## Documentation

See [docs/LOCAL_BUILD_TESTING.md](../../docs/LOCAL_BUILD_TESTING.md) for full documentation.

## CI Equivalence

These containers match the GitHub Actions runners:

| Container | CI Runner | Purpose |
|-----------|-----------|---------|
| `linux-builder` | `ubuntu-latest` | DEB/RPM/AppImage packages |
| `windows-cross-builder` | Cross-compilation | Binary testing (full Windows installers require Windows host) |

## Notes

- **macOS builds**: Not supported in containers due to Apple licensing. Use GitHub Actions or native macOS.
- **Windows installers**: Cross-compilation builds binaries only, not MSI/NSIS installers. Use CI for full Windows testing.
- **Performance**: Docker on Windows/macOS runs in VM, slower than native Linux. Use WSL2 backend on Windows for better performance.
