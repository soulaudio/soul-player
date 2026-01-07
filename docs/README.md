# Soul Player Documentation

Welcome to the Soul Player documentation! This directory contains comprehensive guides, architecture documents, and implementation plans.

---

## Table of Contents

- [Getting Started](#getting-started)
- [Architecture](#architecture)
- [Development](#development)
- [Testing](#testing)
- [Deployment & Release](#deployment--release)
- [Future Plans](#future-plans)

---

## Getting Started

### Quick Setup Guides

- **[SQLx Setup](./SQLX_SETUP.md)** - Configure SQLx for database development and compilation
- **[Linux Build Setup](./LINUX_BUILD_SETUP.md)** - Build Soul Player on Linux systems

### Core Documentation

- **[Conventions](./CONVENTIONS.md)** - Code style, naming conventions, and best practices
- **[Folder Structure](./FOLDER_STRUCTURE.md)** - Monorepo organization and crate responsibilities

---

## Architecture

### System Design

- **[Architecture Overview](./ARCHITECTURE.md)** - High-level system architecture and design principles
- **[Audio Engine Implementation](./AUDIO_ENGINE_IMPLEMENTATION.md)** - Audio processing pipeline details
- **[Playback System](./PLAYBACK_SYSTEM.md)** - Queue management, shuffle, and playback control

### Platform-Specific

- **[ESP32 Manual Testing](./ESP32_MANUAL_TESTING.md)** - Embedded platform testing procedures
- **[Playback Tauri Integration](./PLAYBACK_TAURI_INTEGRATION.md)** - Desktop app audio integration

---

## Development

### Implementation Guides

- **[Audio Engine Tests](./AUDIO_ENGINE_TESTS.md)** - Audio processing test coverage
- **[Playback Implementation Complete](./PLAYBACK_IMPLEMENTATION_COMPLETE.md)** - Playback feature status
- **[Playback Testing](./PLAYBACK_TESTING.md)** - Playback system test suite

### Testing

- **[Testing Guide](./TESTING.md)** - Testing strategy, patterns, and best practices
- **[Testing Report](./TESTING_REPORT.md)** - Test coverage and quality metrics

---

## Deployment & Release

### Release Management

- **[Release Strategy](./RELEASE_STRATEGY.md)** - Versioning, release cadence, and process
- **[Release Workflow Discussion](./RELEASE_WORKFLOW_DISCUSSION.md)** - CI/CD pipeline design
- **[Release Quickstart](./RELEASE_QUICKSTART.md)** - Step-by-step release checklist
- **[Release Testing Matrix](./RELEASE_TESTING_MATRIX.md)** - Cross-platform testing requirements

### Packaging

- **[Linux Packaging](./LINUX_PACKAGING.md)** - Debian/RPM packaging and distribution

---

## Future Plans

### Soul Services Platform

- **[Soul Services Plan](./SOUL_SERVICES_PLAN.md)** ⭐ **NEW** - Comprehensive architecture and implementation plan for the separate Soul Services platform
  - Subscription-based metadata enrichment, discovery, and lyrics service
  - OAuth 2.0 authentication and Stripe integration
  - MusicBrainz, AcoustID, Genius API integrations
  - Self-hosted option with BYOK (Bring Your Own Keys)
  - PostgreSQL backend with multi-tenant architecture
  - 12-week implementation roadmap

---

## Additional Resources

### Subdirectories

- **[architecture/](./architecture/)** - Detailed architecture diagrams and specifications
- **[deployment/](./deployment/)** - Deployment configurations and infrastructure
- **[development/](./development/)** - Development tools and workflows

---

## Contributing to Documentation

When adding new documentation:

1. **Naming Convention**: Use `SCREAMING_SNAKE_CASE.md` for consistency
2. **Update This README**: Add your document to the appropriate section
3. **Link Related Docs**: Cross-reference related documents
4. **Keep It Current**: Update docs when implementation changes

### Documentation Templates

- Architecture documents should include: Overview, Design Decisions, Implementation Details, Trade-offs
- Implementation guides should include: Prerequisites, Step-by-step Instructions, Verification Steps, Troubleshooting
- Testing documents should include: Scope, Test Cases, Coverage Goals, Known Issues

---

## Quick Links

- [Main README](../README.md) - Project overview and quick start
- [CLAUDE.md](../CLAUDE.md) - Instructions for Claude Code when working with this codebase
- [Cargo Workspace](../Cargo.toml) - Root workspace configuration
- [Moon Configuration](../.moon/workspace.yml) - Task runner setup

---

**Last Updated**: 2026-01-06

For questions or suggestions about documentation, please open an issue on GitHub.
