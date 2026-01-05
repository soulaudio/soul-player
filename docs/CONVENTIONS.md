# Soul Player - Coding Conventions & Best Practices

## Rust Edition & Toolchain
- **Edition**: 2021
- **MSRV**: 1.75+ (for latest features)
- **Toolchain**: Stable (upgrade quarterly)

---

## Code Style

### Formatting
- Use `rustfmt` with default settings
- Run `cargo fmt --all` before commits
- CI enforces formatting (blocks PRs)

### Linting
- Use `clippy` with strict settings:
  ```bash
  cargo clippy --all-targets --all-features -- -D warnings
  ```
- Custom clippy config in `Cargo.toml`:
  ```toml
  [workspace.lints.clippy]
  all = "deny"
  pedantic = "warn"
  cargo = "warn"
  ```

### Naming Conventions
- **Crates**: `kebab-case` (e.g., `soul-player-desktop`)
- **Modules**: `snake_case` (e.g., `audio_engine`)
- **Types**: `PascalCase` (e.g., `AudioDecoder`)
- **Functions/Variables**: `snake_case` (e.g., `decode_audio`)
- **Constants**: `SCREAMING_SNAKE_CASE` (e.g., `MAX_BUFFER_SIZE`)

---

## Project Structure

### Crate Organization
```
crates/
├── soul-core/           # Core types and traits (minimal dependencies)
├── soul-storage/        # Database and persistence
├── soul-audio/          # Audio decoding and playback
├── soul-metadata/       # Tag reading and library scanning
├── soul-discovery/      # Music discovery services
├── soul-sync/           # Client-server sync protocol
├── soul-server/         # Multi-user server binary
├── soul-player-desktop/ # Tauri desktop app
└── soul-player-esp32/   # ESP32-S3 firmware
```

### Module Organization
```rust
// Each crate follows this structure:
src/
├── lib.rs              # Public API exports
├── error.rs            # Error types
├── config.rs           # Configuration
├── types/              # Domain types
│   ├── mod.rs
│   ├── track.rs
│   └── playlist.rs
└── services/           # Business logic
    ├── mod.rs
    └── player.rs
```

---

## Dependency Management

### General Rules
- Minimize dependencies (evaluate alternatives)
- Prefer pure Rust crates
- Avoid deprecated/unmaintained crates
- Document why each dependency is needed

### Version Pinning
- Use `~` for patch updates: `serde = "~1.0.195"`
- Lock major versions in workspace `Cargo.toml`
- Regular dependency audits: `cargo update && cargo audit`

### Feature Flags
Use features to reduce compilation for embedded:
```toml
[features]
default = ["desktop"]
desktop = ["cpal", "tauri"]
embedded = ["awedio_esp32"]
server = ["axum", "tokio"]
```

---

## Error Handling

### Error Types
- Use `thiserror` for library errors
- Use `anyhow` for application errors (binaries only)

### Example
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioError {
    #[error("Failed to decode audio: {0}")]
    DecodeFailed(String),

    #[error("Unsupported format: {format}")]
    UnsupportedFormat { format: String },

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

// Usage
pub fn decode_track(path: &Path) -> Result<AudioBuffer, AudioError> {
    // ...
}
```

### Panic Policy
- **NEVER** panic in library code
- Use `Result` or `Option` for recoverable errors
- Only panic for programmer errors (e.g., unreachable code)
- Document all potential panics in doc comments

---

## Async & Concurrency

### Async Runtime
- **Desktop/Server**: Tokio (full features)
- **ESP32-S3**: Embassy (embedded-friendly)

### Guidelines
- Use async for I/O bound operations
- Use threads for CPU-bound work (audio processing)
- Avoid blocking in async contexts
- Use `tokio::spawn_blocking` for heavy compute

### Channel Usage
- **MPSC**: `tokio::sync::mpsc` for async
- **Crossbeam**: For audio thread communication (lock-free)
- Document channel capacity and backpressure handling

---

## Testing Standards

### Quality Over Quantity
- **NO shallow tests** (e.g., testing getters/setters)
- Focus on:
  - Business logic correctness
  - Edge cases and error paths
  - Integration between components
  - API contracts

### Test Organization
```rust
// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_meaningful_behavior() {
        // Arrange
        let decoder = AudioDecoder::new();

        // Act
        let result = decoder.decode(&sample_data);

        // Assert
        assert!(result.is_ok());
    }
}

// Integration tests in tests/
// tests/storage_integration.rs
```

### Property-Based Testing
Use `proptest` for complex logic:
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_eq_preserves_energy(gain: f32) {
        // Test that EQ doesn't amplify beyond gain
    }
}
```

### Testcontainers
- Use for SQLite integration tests
- Use for server multi-user tests
- **NEVER** use in-memory SQLite for tests
- Clean up containers after tests

---

## Documentation

### Inline Documentation
- All public items MUST have doc comments
- Use `///` for doc comments
- Include examples where helpful

```rust
/// Decodes an audio file from the given path.
///
/// # Arguments
/// * `path` - Path to the audio file
///
/// # Returns
/// `AudioBuffer` containing decoded samples
///
/// # Errors
/// Returns `AudioError::DecodeFailed` if decoding fails
///
/// # Example
/// ```
/// let buffer = decode_audio(Path::new("song.mp3"))?;
/// ```
pub fn decode_audio(path: &Path) -> Result<AudioBuffer, AudioError> {
    // ...
}
```

### README Structure
Each crate needs `README.md`:
- Purpose and scope
- Usage examples
- Feature flags
- Architecture notes (if complex)

---

## Platform-Specific Code

### Conditional Compilation
```rust
#[cfg(target_os = "espidf")]
use awedio_esp32 as audio_backend;

#[cfg(not(target_os = "espidf"))]
use cpal as audio_backend;
```

### Trait Abstraction
Prefer traits over `cfg` when possible:
```rust
pub trait AudioOutput {
    fn play(&mut self, buffer: &AudioBuffer) -> Result<()>;
}

// Platform-specific implementations
impl AudioOutput for CpalOutput { /* ... */ }
impl AudioOutput for EspOutput { /* ... */ }
```

---

## Performance Guidelines

### Audio Thread
- **NEVER** allocate in audio callback
- Pre-allocate buffers
- Use lock-free data structures
- Avoid mutex/locks in hot path

### Memory Management
- Use `Box` for large structs
- Use `Arc` for shared ownership
- Minimize cloning of large data
- Profile with `cargo flamegraph`

### Embedded Constraints
- Limit stack usage (<32KB)
- Minimize heap allocations
- Use static buffers where possible
- Measure binary size: `cargo bloat`

---

## Security Practices

### Input Validation
- Validate all external input (files, network, user)
- Sanitize file paths (no path traversal)
- Limit resource consumption (file size, connections)

### Secrets Management
- **NEVER** hardcode secrets
- Use environment variables
- Document required env vars
- Use `secrecy` crate for sensitive data

### Dependency Audits
- Run `cargo audit` on every PR (CI enforced)
- Update vulnerable dependencies immediately
- Subscribe to security advisories

---

## Git Workflow

### Commit Messages
Follow Conventional Commits:
```
feat(audio): add FLAC decoder support
fix(storage): resolve playlist sync race condition
docs(readme): update build instructions
test(metadata): add property tests for tag parsing
chore(deps): update symphonia to 0.5.4
```

### Branch Strategy
- `main` - production-ready code
- `develop` - integration branch
- `feature/*` - new features
- `fix/*` - bug fixes

### PR Requirements
- All tests pass
- Clippy warnings resolved
- Formatted with rustfmt
- Security audit passes
- Code coverage reported (no strict threshold, aim 50-60%)

---

## CI/CD Standards

### Blocking Checks
PRs are blocked if:
- ❌ Tests fail
- ❌ Clippy errors/warnings
- ❌ Rustfmt differences
- ❌ Security vulnerabilities (`cargo audit`)

### Non-Blocking Checks
- ⚠️ Code coverage (report only, aim 50-60%)
- ⚠️ Performance benchmarks (informational)

### Build Matrix
- **OS**: Linux, macOS, Windows
- **Arch**: x64, ARM64
- **Targets**: Desktop, Server, ESP32-S3

---

## Embedded-Specific Guidelines

### Resource Budgets (ESP32-S3)
- **Flash**: <2MB for firmware
- **SRAM**: <256KB static allocation
- **Heap**: <512KB dynamic allocation
- **Stack**: <32KB per task

### Hardware Abstraction
- Use Embassy HAL for peripherals
- Minimize hardware-specific code in core logic
- Abstract behind traits for testability

### Power Management
- Use sleep modes when idle
- Minimize WiFi active time
- Profile power consumption
- Target 8+ hours battery life

---

## Release Checklist

Before tagging a release:
- [ ] All tests pass on all platforms
- [ ] Changelog updated (CHANGELOG.md)
- [ ] Version bumped (Cargo.toml)
- [ ] Documentation updated
- [ ] Security audit clean
- [ ] Performance benchmarks stable
- [ ] Breaking changes documented

---

## Additional Resources

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Embedded Rust Book](https://doc.rust-lang.org/embedded-book/)
- [Tauri Best Practices](https://v2.tauri.app/develop/tests/)
- [ESP-RS Documentation](https://docs.esp-rs.org/)
