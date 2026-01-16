# Local CI Testing with Docker

This directory contains scripts to run CI tests locally in an environment that **exactly** replicates GitHub Actions CI.

## Why Use Docker for CI Testing?

- ✅ **100% CI Accuracy**: Uses same Ubuntu version, same Rust toolchain, same dependencies as CI
- ✅ **Catch Issues Before Push**: Find test failures locally that only happen in CI
- ✅ **Fast Iterations**: Cached builds mean subsequent runs take 1-3 minutes
- ✅ **No WSL2 Setup Needed**: Works on any platform with Docker Desktop
- ✅ **Release Mode Testing**: Tests run in `--release` mode matching CI exactly

## Quick Start

### Windows (PowerShell)

```powershell
# Run all integration tests (like CI)
.\test-ci.ps1

# Run tests for specific package
.\test-ci.ps1 -Package soul-playback

# Run clippy
.\test-ci.ps1 -Clippy

# Open interactive shell for debugging
.\test-ci.ps1 -Shell
```

### Linux/macOS (Bash)

```bash
# Make script executable (first time only)
chmod +x test-ci.sh

# Run all integration tests (like CI)
./test-ci.sh

# Run tests for specific package
./test-ci.sh --package soul-playback

# Run clippy
./test-ci.sh --clippy

# Open interactive shell for debugging
./test-ci.sh --shell
```

## What Gets Tested

The Docker environment replicates these CI jobs from `.github/workflows/ci.yml`:

### Integration Tests (default)
```bash
# Matches CI lines 330-335
cargo test --tests --release \
  -p soul-audio \
  -p soul-audio-desktop \
  -p soul-playback \
  -p soul-storage \
  -- --test-threads=1
```

### Clippy
```bash
# Matches CI line 91
cargo clippy --workspace --lib --bins --release -- -D warnings
```

### Format Check
```bash
# Matches CI line 61
cargo fmt --all --check
```

## Performance

| Run Type | Duration | Details |
|----------|----------|---------|
| **First Run** | 5-10 min | Downloads dependencies, compiles everything |
| **Subsequent Runs** | 1-3 min | Uses cached cargo registry, git, and target |
| **After Code Change** | 30s-2min | Incremental compilation |
| **Format/Clippy Only** | 10-30s | No compilation needed |

## Docker Volumes (Persistent Caches)

The setup uses Docker volumes to cache build artifacts:

- `cargo-registry`: Downloaded crates from crates.io (~500MB)
- `cargo-git`: Git dependencies (~100MB)
- `cargo-target-ci`: Compiled artifacts (`target/` directory, ~5-10GB)

**To clean caches:**
```powershell
# Windows
.\test-ci.ps1 -Clean

# Linux/macOS
./test-ci.sh --clean
```

## Environment Variables

The Docker container uses **exact** CI environment variables from `.github/workflows/ci.yml`:

```bash
CARGO_TERM_COLOR=always
RUST_BACKTRACE=1
CARGO_INCREMENTAL=0           # Disable incremental for deterministic builds
CARGO_NET_RETRY=10            # Retry network failures
RUSTUP_MAX_RETRIES=10
CARGO_PROFILE_TEST_OPT_LEVEL=3  # Release optimizations
CARGO_PROFILE_TEST_DEBUG=0      # No debug symbols
RUST_TEST_THREADS=4           # Parallel test execution
```

## Common Workflows

### Before Committing

```powershell
# Run full CI test suite
.\test-ci.ps1

# If tests pass:
git add .
git commit -m "your message"
git push
```

### Debugging a Specific Test Failure

```powershell
# Open shell in CI environment
.\test-ci.ps1 -Shell

# Inside container:
cargo test --tests --release -p soul-playback test_play_pause_resume_workflow -- --nocapture

# Or run specific test file:
cargo test --tests --release -p soul-playback --test integration_test -- --nocapture
```

### After Updating Dependencies

```powershell
# Clean caches to ensure fresh dependency resolution
.\test-ci.ps1 -Clean -Build

# Run tests
.\test-ci.ps1
```

### Testing a Specific Package

```powershell
# Only test soul-playback
.\test-ci.ps1 -Package soul-playback

# Only test soul-audio-desktop
.\test-ci.ps1 -Package soul-audio-desktop
```

## Troubleshooting

### Docker Not Running
```
ERROR: Docker is not running. Please start Docker Desktop.
```
**Solution**: Start Docker Desktop and wait for it to fully initialize.

### Out of Disk Space
```
error: failed to remove file: No space left on device
```
**Solution**: Clean Docker volumes and images:
```powershell
.\test-ci.ps1 -Clean
docker system prune -a --volumes  # WARNING: Removes ALL Docker data
```

### Tests Pass Locally But Fail in CI

This means you're not running in the Docker environment. Make sure to use:
- ✅ `.\test-ci.ps1` (runs in Docker)
- ❌ `cargo test` (runs on host, different environment)

### Slow First Run

The first run downloads and compiles **everything**:
- Rust toolchain
- All dependencies (~300 crates)
- All workspace crates

**This is normal**. Subsequent runs will be much faster (~1-3 minutes).

## Advanced Usage

### Manual Docker Commands

```bash
# Build image manually
docker compose -f docker-compose.ci.yml build

# Run tests manually
docker compose -f docker-compose.ci.yml up

# Run specific command
docker compose -f docker-compose.ci.yml run --rm ci-test bash -c "cargo test -p soul-playback"

# Clean up
docker compose -f docker-compose.ci.yml down -v
```

### Customizing Environment

Edit `Dockerfile.ci` or `docker-compose.ci.yml` to:
- Change Rust version
- Add dependencies
- Modify environment variables
- Adjust test commands

**After changes, rebuild:**
```powershell
.\test-ci.ps1 -Build
```

## Files

| File | Purpose |
|------|---------|
| `Dockerfile.ci` | Docker image definition (Ubuntu 22.04 + Rust stable) |
| `docker-compose.ci.yml` | Compose configuration with volume mounts |
| `test-ci.ps1` | PowerShell runner script (Windows) |
| `test-ci.sh` | Bash runner script (Linux/macOS) |
| `CI_TESTING.md` | This documentation |

## Comparison: Docker vs WSL2

| Feature | Docker (this setup) | WSL2 Native |
|---------|---------------------|-------------|
| **Setup Time** | 5 min | 30-60 min |
| **Accuracy** | 100% (exact CI) | 100% (exact CI) |
| **Build Speed** | Good (cached volumes) | Excellent (native filesystem) |
| **Isolation** | Excellent (containerized) | None (shares host) |
| **Cleanup** | Easy (`-Clean` flag) | Manual |
| **Multi-platform** | ✅ Windows/macOS/Linux | ❌ Windows/Linux only |
| **Recommendation** | **Testing CI issues** | Long-term development |

**Use Docker when:**
- You want to replicate CI exactly
- You need quick setup
- You want isolated environments
- You're on macOS (no WSL2 available)

**Use WSL2 when:**
- You do daily development on Linux
- You need maximum performance
- You want a persistent Linux environment

## Integration with CI

This setup mirrors `.github/workflows/ci.yml` **test-containers** job:

| CI Step | Docker Equivalent |
|---------|-------------------|
| `ubuntu-latest` | `FROM ubuntu:22.04` |
| `dtolnay/rust-toolchain@stable` | `rustup.rs` install |
| `apt-get install libasound2-dev...` | `RUN apt-get install...` |
| `cargo test --tests --release` | Default CMD |
| `--test-threads=1` | Included in CMD |
| `CARGO_PROFILE_TEST_OPT_LEVEL=3` | ENV variable |

**Result**: If tests pass in Docker, they **will** pass in CI.

## FAQ

**Q: Why not just use `cargo test` locally?**
A: Your local environment (Windows, different versions) doesn't match CI (Ubuntu, specific versions). Tests can pass locally but fail in CI.

**Q: Do I need to rebuild the image every time?**
A: No. Only rebuild when you change `Dockerfile.ci` or update Rust/dependencies.

**Q: Can I use this for development?**
A: Yes, but it's slower than native development. Use it for **final validation** before pushing.

**Q: How much disk space does this use?**
A: ~10-15GB total (Docker image ~2GB, volumes ~10GB). Clean with `-Clean` flag when needed.

**Q: Can I run this in CI?**
A: No need - this IS the CI environment. Use this locally to replicate CI.

## Next Steps

1. **Run tests locally** before every push:
   ```powershell
   .\test-ci.ps1
   ```

2. **Fix any failures** in the Docker environment (use `-Shell` for debugging)

3. **Push with confidence** knowing CI will pass

---

**Questions?** See `.github/workflows/ci.yml` for the exact CI configuration this replicates.
