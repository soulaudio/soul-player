# Soul Player Documentation

Welcome to the Soul Player documentation! This directory contains comprehensive guides, architecture documents, and implementation plans.

---

## Table of Contents

- [Getting Started](#getting-started)
- [Architecture](#architecture)
- [Development](#development)
- [Future Plans](#future-plans)

---

## Getting Started

### Quick Setup Guides

- **[SQLx Setup](./SQLX_SETUP.md)** - Configure SQLx for database development and compilation
- **[SQLx Troubleshooting](./SQLX_TROUBLESHOOTING.md)** - Common SQLx issues and solutions

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

### Architecture Subdirectory

- **[Audio Abstraction](./architecture/AUDIO_ABSTRACTION.md)** - Audio DI pattern and platform abstraction
- **[Frontend Architecture](./architecture/FRONTEND_ARCHITECTURE.md)** - React architecture and component design
- **[Multi-Source Architecture](./architecture/MULTI_SOURCE_ARCHITECTURE.md)** - Multiple music sources support
- **[Sync Strategy](./architecture/SYNC_STRATEGY.md)** - Server synchronization protocol
- **[Offline Mode](./architecture/OFFLINE_MODE.md)** - Offline-first design patterns
- **[Architecture Decisions](./architecture/ARCHITECTURE_DECISIONS.md)** - Key architectural decisions and rationale

---

## Development

### Testing

- **[Testing Guide](./TESTING.md)** - Testing strategy, patterns, and best practices

### Deployment

- **[CI/CD](./deployment/CI_CD.md)** - Continuous integration and deployment

### Mobile

- **[Mobile Setup](./development/MOBILE_SETUP.md)** - Mobile development environment setup

---

## Future Plans

### Soul Services Platform

- **[Soul Services Plan](./SOUL_SERVICES_PLAN.md)** - Comprehensive architecture and implementation plan for the separate Soul Services platform
  - Subscription-based metadata enrichment, discovery, and lyrics service
  - OAuth 2.0 authentication and Stripe integration
  - MusicBrainz, AcoustID, Genius API integrations
  - Self-hosted option with BYOK (Bring Your Own Keys)

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

---

## Quick Links

- [Main README](../README.md) - Project overview and quick start
- [CLAUDE.md](../CLAUDE.md) - Instructions for Claude Code when working with this codebase
- [ROADMAP.md](../ROADMAP.md) - Development phases and progress
- [CONTRIBUTING.md](../CONTRIBUTING.md) - How to contribute

---

**Last Updated**: 2026-01-10
