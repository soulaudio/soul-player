# Soul Services - Architecture & Implementation Plan

**Status**: Planning Phase
**Target Implementation**: Separate Repository
**Last Updated**: 2026-01-06

---

## Executive Summary

**Soul Services** is a separate, subscription-based platform that provides premium music metadata enrichment, discovery, and enhancement features for Soul Player and potentially other music applications.

### Why Separate from Soul Player?

1. **Business Model**: Soul Player is open-source; Soul Services is a sustainable revenue stream
2. **Scalability**: Independent scaling of compute-intensive services (fingerprinting, scraping)
3. **Licensing**: Closed-source service protects business logic while keeping player open
4. **Infrastructure**: PostgreSQL + cloud hosting vs SQLite + local-first for player

### Core Value Proposition

- **For Users**: Automatic metadata enrichment, lyrics, discovery without managing API keys
- **For Self-Hosters**: Option to bring your own API keys and host the service yourself
- **For Developers**: Clean API for integrating music intelligence into any app

---

## Business Model

### Subscription Tiers

| Tier | Price | Features | Target Audience |
|------|-------|----------|-----------------|
| **Pro** | $5-10/month | Unlimited metadata enrichment, lyrics, basic discovery, album artwork | Individual power users |
| **Audiophile** | $15-20/month | Everything in Pro + audio fingerprinting (AcoustID), high-res artwork, advanced recommendations, priority processing | Audio enthusiasts, DJs, collectors |
| **Self-Hosted** | Free | All features if you provide your own API keys (MusicBrainz, AcoustID, Genius, etc.) | Privacy-conscious users, developers |

### Revenue Projections (Hypothetical)

- 1,000 users → $5-15k MRR
- Margins: 60-70% (API costs are main expense)
- Break-even: ~100 paying users

---

## System Architecture

### High-Level Overview

```
┌─────────────────┐
│   Soul Player   │ (Open Source - Local First)
│   (Desktop)     │
└────────┬────────┘
         │ OAuth 2.0 + PKCE
         │ REST API
         ▼
┌─────────────────────────────────────────┐
│         Soul Services Platform          │ (Closed Source - Cloud/Self-Hosted)
│  ┌──────────────────────────────────┐  │
│  │     soul-discovery (MVP)         │  │
│  │  - Metadata Enrichment           │  │
│  │  - Audio Fingerprinting          │  │
│  │  - Lyrics (synced + unsynced)    │  │
│  │  - Discovery/Recommendations     │  │
│  │  - Bandcamp/Discogs Scraping     │  │
│  └──────────────────────────────────┘  │
│                                         │
│  ┌──────────────────────────────────┐  │
│  │     soul-auth (OAuth Provider)   │  │
│  │  - User Registration/Login       │  │
│  │  - Stripe Subscription Webhooks  │  │
│  │  - Device Management             │  │
│  │  - API Key Management            │  │
│  └──────────────────────────────────┘  │
│                                         │
│  ┌──────────────────────────────────┐  │
│  │     PostgreSQL Database          │  │
│  │  - Users & Subscriptions         │  │
│  │  - Cached Metadata               │  │
│  │  - Fingerprints & Lyrics         │  │
│  │  - Usage Analytics               │  │
│  └──────────────────────────────────┘  │
└─────────────────────────────────────────┘
         │
         │ External APIs
         ▼
┌─────────────────────────────────────────┐
│      External Services                  │
│  - MusicBrainz API                      │
│  - AcoustID API                         │
│  - Genius API (Lyrics)                  │
│  - Musixmatch API (Lyrics)              │
│  - Bandcamp (Scraping)                  │
│  - Discogs API                          │
│  - Stripe (Payments)                    │
└─────────────────────────────────────────┘
```

### Microservices (Future)

Initial MVP combines everything in one deployable. Future split:

1. **soul-auth**: Authentication, subscriptions, user management
2. **soul-discovery**: All music intelligence features
3. **soul-gateway**: Rate limiting, request routing, API versioning
4. **soul-worker**: Background jobs (scraping, batch processing)

---

## soul-discovery Service Specification

### Core Features

#### 1. Audio Fingerprinting (AcoustID)

**Purpose**: Identify unknown tracks, auto-tag incorrectly labeled files

**Flow**:
```
1. Soul Player sends audio fingerprint (generated locally using chromaprint)
2. soul-discovery queries AcoustID API
3. Returns MusicBrainz Recording ID + basic metadata
4. Optionally enriches with full MusicBrainz data
```

**API Endpoint**:
```http
POST /api/v1/fingerprint
Content-Type: application/json
Authorization: Bearer <jwt_token>

{
  "fingerprint": "AQADtN...",  // Chromaprint fingerprint
  "duration": 245,              // Track duration in seconds
  "enrich": true                // Auto-fetch full metadata
}

Response:
{
  "match_score": 0.98,
  "track": {
    "musicbrainz_id": "abc-123",
    "title": "Bohemian Rhapsody",
    "artist": "Queen",
    "album": "A Night at the Opera",
    "year": 1975,
    "isrc": "GBUM71029604"
  }
}
```

**Cost**: AcoustID charges per lookup (~$0.001/lookup). Audiophile tier only.

#### 2. Metadata Enrichment (MusicBrainz)

**Purpose**: Fetch comprehensive artist/album data beyond basic tags

**Data Retrieved**:
- **Artists**: Biography, formation date, country, genre tags, similar artists
- **Albums**: Release date, label, catalog number, cover art (multiple sizes), track listing
- **Tracks**: Recording date, composers, lyricists, ISRC codes
- **Relationships**: Band members, producers, featured artists

**API Endpoints**:
```http
GET /api/v1/artist/:id/enrich
GET /api/v1/album/:id/enrich
GET /api/v1/track/:musicbrainz_id
```

**Caching**: 90-day cache for immutable data (older releases), 7-day for recent releases

#### 3. Lyrics Fetching

**Sources** (in priority order):
1. **Embedded LRC files** (if user uploaded)
2. **Genius API** (official, reliable, but rate-limited)
3. **Musixmatch API** (synced lyrics available)
4. **LRCLIB** (community-driven, free)
5. **Web scraping** (fallback, respects robots.txt)

**Features**:
- Unsynced lyrics (plain text)
- Synced lyrics (LRC format with timestamps)
- Multi-language support
- Romanization for non-Latin scripts

**API Endpoints**:
```http
GET /api/v1/lyrics
  ?artist=Queen
  &title=Bohemian+Rhapsody
  &album=A+Night+at+the+Opera
  &duration=354
  &synced=true

Response:
{
  "lyrics": "[00:00.00] Is this the real life?\n[00:04.50] Is this just fantasy?...",
  "synced": true,
  "source": "musixmatch",
  "language": "en"
}
```

#### 4. Discovery & Recommendations

**Algorithms**:
- **Similar Artists**: MusicBrainz relationships + collaborative filtering
- **Genre Exploration**: Curated genre trees from MusicBrainz + Last.fm tags
- **New Releases**: Track followed artists' latest releases via MusicBrainz
- **Hidden Gems**: Recommend under-played tracks from user's library

**API Endpoints**:
```http
GET /api/v1/discover/similar-artists?artist_id=123
GET /api/v1/discover/new-releases?user_id=456
GET /api/v1/discover/genre/:genre_name
```

#### 5. Bandcamp & Discogs Integration

**Bandcamp**:
- Artist discovery (trending, tags)
- Album scraping (when MusicBrainz lacks data for obscure releases)
- Purchase link integration

**Discogs**:
- Vinyl/physical release info (pressing details, catalog numbers)
- Marketplace pricing data (for collectors)
- Artist discography completion

**Rate Limiting**: Aggressive caching + respect for site policies (no mass scraping)

---

## Authentication & Authorization

### OAuth 2.0 with PKCE Flow

**Why PKCE?** Desktop apps can't securely store client secrets. PKCE (Proof Key for Code Exchange) adds security without requiring secrets.

**Flow**:
```
1. Soul Player generates code_verifier + code_challenge
2. Opens browser to: https://services.soul.audio/oauth/authorize
3. User logs in / signs up
4. Redirects to soul://oauth/callback?code=xyz
5. Soul Player exchanges code + code_verifier for access_token
6. Stores access_token securely (OS keychain)
7. Includes in all API requests: Authorization: Bearer <token>
```

**Token Lifecycle**:
- Access tokens: 1-hour expiry
- Refresh tokens: 30-day expiry
- Automatic refresh in background

### Device Management

Users can manage devices in web portal:
- View active sessions
- Revoke access per device
- See last used timestamp

---

## Database Schema (PostgreSQL)

### Core Tables

```sql
-- Users & Authentication
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    username VARCHAR(50) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,  -- bcrypt
    email_verified BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE oauth_clients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    device_name VARCHAR(100),  -- "John's MacBook Pro"
    device_type VARCHAR(20),   -- "desktop", "mobile", "web"
    last_used_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE oauth_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_id UUID REFERENCES oauth_clients(id) ON DELETE CASCADE,
    access_token VARCHAR(255) UNIQUE NOT NULL,
    refresh_token VARCHAR(255) UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Subscriptions (Stripe)
CREATE TABLE subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    stripe_customer_id VARCHAR(100) UNIQUE,
    stripe_subscription_id VARCHAR(100) UNIQUE,
    tier VARCHAR(20) NOT NULL,  -- "pro", "audiophile"
    status VARCHAR(20) NOT NULL, -- "active", "canceled", "past_due"
    current_period_end TIMESTAMPTZ NOT NULL,
    cancel_at_period_end BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Self-Hosted API Keys
CREATE TABLE api_keys (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    service VARCHAR(50) NOT NULL,  -- "musicbrainz", "acoustid", "genius"
    encrypted_key TEXT NOT NULL,   -- AES-256 encrypted
    is_valid BOOLEAN DEFAULT TRUE,
    last_checked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Metadata Cache
CREATE TABLE artist_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    musicbrainz_id UUID UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    sort_name VARCHAR(255),
    bio TEXT,
    country VARCHAR(2),
    formed_date DATE,
    genre_tags JSONB,  -- ["rock", "progressive rock"]
    similar_artists JSONB,  -- [{"id": "...", "name": "..."}, ...]
    cached_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE album_metadata (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    musicbrainz_id UUID UNIQUE NOT NULL,
    title VARCHAR(500) NOT NULL,
    artist_id UUID REFERENCES artist_metadata(musicbrainz_id),
    release_date DATE,
    label VARCHAR(255),
    catalog_number VARCHAR(100),
    cover_art_url TEXT,
    cover_art_hires_url TEXT,
    track_listing JSONB,
    cached_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE lyrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    track_fingerprint VARCHAR(50) NOT NULL,  -- SHA-256 of artist+title+album
    artist VARCHAR(255) NOT NULL,
    title VARCHAR(255) NOT NULL,
    album VARCHAR(255),
    lyrics TEXT NOT NULL,
    synced BOOLEAN DEFAULT FALSE,
    language VARCHAR(5) DEFAULT 'en',
    source VARCHAR(50),  -- "genius", "musixmatch", "lrclib"
    cached_at TIMESTAMPTZ DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_lyrics_fingerprint ON lyrics(track_fingerprint);

-- Usage Tracking (Rate Limiting)
CREATE TABLE api_usage (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    endpoint VARCHAR(100) NOT NULL,
    request_count INTEGER DEFAULT 1,
    date DATE NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_usage_user_endpoint_date ON api_usage(user_id, endpoint, date);

-- AcoustID Fingerprints (Audiophile tier only)
CREATE TABLE fingerprints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    fingerprint TEXT NOT NULL,
    duration INTEGER NOT NULL,
    matched_recording_id UUID,  -- MusicBrainz recording ID
    match_score DECIMAL(3,2),
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Indexes

```sql
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_oauth_tokens_access ON oauth_tokens(access_token);
CREATE INDEX idx_subscriptions_user_status ON subscriptions(user_id, status);
CREATE INDEX idx_artist_metadata_mb_id ON artist_metadata(musicbrainz_id);
CREATE INDEX idx_album_metadata_mb_id ON album_metadata(musicbrainz_id);
CREATE INDEX idx_fingerprints_user_created ON fingerprints(user_id, created_at);
```

---

## API Design

### Base URL

- Production: `https://services.soul.audio`
- Self-Hosted: `http://localhost:3001` (default)

### Versioning

All endpoints prefixed with `/api/v1/` for future-proofing.

### Authentication

```http
Authorization: Bearer <jwt_access_token>
```

### Rate Limiting

| Tier | Requests/Minute | Fingerprints/Month | Cache TTL |
|------|-----------------|-----------------------|-----------|
| Pro | 60 | N/A | 7 days |
| Audiophile | 120 | 1,000 | 90 days |
| Self-Hosted | Unlimited* | Unlimited* | Configurable |

*Self-hosted limits based on your API key quotas

### Error Responses

```json
{
  "error": {
    "code": "RATE_LIMIT_EXCEEDED",
    "message": "You have exceeded your monthly fingerprint quota (1000)",
    "details": {
      "reset_at": "2026-02-01T00:00:00Z",
      "current_usage": 1001,
      "limit": 1000
    }
  }
}
```

### Core Endpoints

#### Authentication

```http
POST   /api/v1/auth/register
POST   /api/v1/auth/login
POST   /api/v1/auth/refresh
POST   /api/v1/auth/logout
GET    /api/v1/auth/me
```

#### OAuth

```http
GET    /oauth/authorize
POST   /oauth/token
POST   /oauth/revoke
```

#### Metadata

```http
GET    /api/v1/artist/:id/enrich
GET    /api/v1/album/:id/enrich
GET    /api/v1/track/:musicbrainz_id
POST   /api/v1/fingerprint
GET    /api/v1/lyrics
```

#### Discovery

```http
GET    /api/v1/discover/similar-artists
GET    /api/v1/discover/new-releases
GET    /api/v1/discover/genre/:name
GET    /api/v1/discover/trending
```

#### Subscription Management

```http
GET    /api/v1/subscription
POST   /api/v1/subscription/create-checkout
POST   /api/v1/subscription/portal
POST   /webhooks/stripe  (Stripe webhooks)
```

#### Self-Hosted API Keys

```http
GET    /api/v1/api-keys
POST   /api/v1/api-keys
DELETE /api/v1/api-keys/:id
POST   /api/v1/api-keys/:id/validate
```

---

## Technology Stack

### Backend

| Component | Technology | Reasoning |
|-----------|------------|-----------|
| **Web Framework** | Axum | Fast, type-safe, integrates with Tokio |
| **Database** | PostgreSQL 16 | JSON support (JSONB), robust, scalable |
| **ORM** | SQLx | Compile-time checked queries, async-first |
| **Auth** | OAuth2 crate + JWT | Standard implementations |
| **HTTP Client** | reqwest | Async, widely used for API calls |
| **Scraping** | scraper (HTML) + headless_chrome | Static pages + JS-rendered content |
| **Caching** | Redis (optional) | Session storage, hot metadata cache |
| **Background Jobs** | tokio-cron-scheduler | Scheduled scraping, cleanup tasks |

### External APIs

| Service | Purpose | Cost Model |
|---------|---------|------------|
| **MusicBrainz** | Artist/album metadata | Free (1 req/sec, rate-limited) |
| **AcoustID** | Audio fingerprinting | $0.001/lookup (paid credits) |
| **Genius** | Lyrics (official) | Free API (rate-limited) |
| **Musixmatch** | Synced lyrics | Paid API (~$50/month for commercial) |
| **LRCLIB** | Community lyrics | Free (donations accepted) |
| **Discogs** | Vinyl/physical data | Free API (rate-limited) |
| **Stripe** | Payment processing | 2.9% + $0.30/transaction |

### Infrastructure

| Component | Options | Recommendation |
|-----------|---------|----------------|
| **Hosting** | Fly.io, Railway, AWS | Fly.io (edge deployment, easy Postgres) |
| **Database** | Fly Postgres, Supabase, AWS RDS | Fly Postgres (integrated) |
| **Object Storage** | S3, Cloudflare R2 | R2 (free egress, cheaper) |
| **CDN** | Cloudflare | Free tier sufficient for images |
| **Monitoring** | Sentry, Honeycomb | Sentry (error tracking) |

### Deployment (Docker)

```dockerfile
# Dockerfile
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin soul-discovery

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/soul-discovery /usr/local/bin/
EXPOSE 3001
CMD ["soul-discovery"]
```

```yaml
# docker-compose.yml
version: '3.8'
services:
  soul-discovery:
    build: .
    ports:
      - "3001:3001"
    environment:
      DATABASE_URL: postgres://user:pass@db:5432/soul_services
      REDIS_URL: redis://redis:6379
      STRIPE_SECRET_KEY: ${STRIPE_SECRET_KEY}
      JWT_SECRET: ${JWT_SECRET}
    depends_on:
      - db
      - redis

  db:
    image: postgres:16
    volumes:
      - postgres_data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: soul_services
      POSTGRES_USER: user
      POSTGRES_PASSWORD: pass

  redis:
    image: redis:7
    volumes:
      - redis_data:/data

volumes:
  postgres_data:
  redis_data:
```

---

## Integration with Soul Player

### Client SDK (Rust Crate)

Publish `soul-services-client` crate for easy integration:

```rust
// In Soul Player's Cargo.toml
[dependencies]
soul-services-client = "0.1"

// Usage in Soul Player
use soul_services_client::{SoulServicesClient, AuthFlow};

#[tokio::main]
async fn main() {
    // Initialize client
    let client = SoulServicesClient::new("https://services.soul.audio");

    // OAuth flow (opens browser)
    let auth = client.authenticate_pkce().await?;

    // Store tokens securely
    keyring::set_password("soul-services", "access_token", &auth.access_token)?;

    // Enrich metadata
    let artist = client.enrich_artist("queen-musicbrainz-id").await?;
    println!("Bio: {}", artist.bio);

    // Fetch lyrics
    let lyrics = client.get_lyrics("Queen", "Bohemian Rhapsody", true).await?;
    if lyrics.synced {
        // Display with timestamps
    }
}
```

### Fallback Handling

Soul Player should gracefully handle service unavailability:

```rust
match client.enrich_artist(id).await {
    Ok(metadata) => display_rich_artist_page(metadata),
    Err(SoulServicesError::Unauthenticated) => {
        // Prompt user to subscribe
        show_subscription_prompt()
    }
    Err(SoulServicesError::NetworkError(_)) => {
        // Show cached data or basic info
        display_basic_artist_info(local_db)
    }
    Err(e) => {
        log::error!("Enrichment failed: {}", e);
        // Continue with local data
    }
}
```

### UI Integration

**Settings Page**:
```
[ Soul Services ]
Status: Connected (Pro Tier)
Email: user@example.com

[ Manage Subscription ] [ Disconnect ]

Features Enabled:
✓ Metadata Enrichment
✓ Lyrics (Synced)
✓ Discovery
✗ Audio Fingerprinting (Upgrade to Audiophile)
```

**Artist Page** (if enriched):
```
Queen
📍 London, United Kingdom
🎸 Formed: 1970

[Rich biography from MusicBrainz]

Similar Artists:
• David Bowie
• Led Zeppelin
• Pink Floyd

[Powered by Soul Services]
```

---

## Implementation Phases

### Phase 1: MVP (Weeks 1-4)

**Week 1-2: Core Infrastructure**
- [ ] Repository setup + monorepo structure
- [ ] PostgreSQL schema + migrations
- [ ] Axum server with basic routing
- [ ] Docker + docker-compose for local dev
- [ ] OAuth 2.0 + PKCE implementation
- [ ] JWT token generation/validation

**Week 3: MusicBrainz Integration**
- [ ] MusicBrainz API client
- [ ] Artist/album enrichment endpoints
- [ ] Response caching (90-day TTL)
- [ ] Rate limiting middleware

**Week 4: Lyrics + Stripe**
- [ ] Lyrics fetching (Genius + LRCLIB)
- [ ] Synced lyrics parsing (LRC format)
- [ ] Stripe subscription webhooks
- [ ] Subscription tier enforcement

**Deliverable**: Working service with metadata + lyrics, deployable to Fly.io

### Phase 2: Advanced Features (Weeks 5-8)

**Week 5: Audio Fingerprinting**
- [ ] AcoustID API integration
- [ ] Fingerprint caching
- [ ] Audiophile tier quota enforcement

**Week 6: Discovery Algorithms**
- [ ] Similar artists (MusicBrainz relationships)
- [ ] New releases tracking
- [ ] Genre exploration endpoints

**Week 7-8: Scraping**
- [ ] Bandcamp scraper (respect rate limits)
- [ ] Discogs API integration
- [ ] Background job scheduler

**Deliverable**: Full feature parity with planned spec

### Phase 3: Polish & Scale (Weeks 9-12)

**Week 9: Client SDK**
- [ ] Publish `soul-services-client` crate
- [ ] Integration examples
- [ ] Error handling best practices

**Week 10: Self-Hosted Support**
- [ ] API key management UI
- [ ] Validation checks for user-provided keys
- [ ] Configuration guide

**Week 11: Monitoring + Optimization**
- [ ] Sentry integration
- [ ] Database query optimization
- [ ] Cache hit rate monitoring

**Week 12: Documentation + Launch**
- [ ] API documentation (OpenAPI/Swagger)
- [ ] Self-hosting guide
- [ ] Pricing page + Stripe checkout flow

---

## Self-Hosted Deployment Guide

### Prerequisites

- Docker + Docker Compose
- Domain with SSL (Let's Encrypt recommended)
- API keys for:
  - Genius API (free tier)
  - AcoustID (if using fingerprinting)
  - (Optional) Musixmatch API

### Quick Start

```bash
# Clone the repository
git clone https://github.com/soulaudio/soul-services
cd soul-services

# Copy environment template
cp .env.example .env

# Edit .env with your API keys
nano .env

# Generate JWT secret
openssl rand -base64 32

# Start services
docker-compose up -d

# Run migrations
docker-compose exec soul-discovery ./migrate

# Access at http://localhost:3001
```

### Environment Variables

```bash
# .env
DATABASE_URL=postgres://user:password@db:5432/soul_services
REDIS_URL=redis://redis:6379
JWT_SECRET=<generate-with-openssl-rand-base64-32>

# External API Keys (Self-Hosted)
MUSICBRAINZ_APP_NAME=SoulServices
MUSICBRAINZ_APP_VERSION=0.1.0
MUSICBRAINZ_CONTACT=your-email@example.com
ACOUSTID_API_KEY=<your-key>
GENIUS_API_TOKEN=<your-token>
MUSIXMATCH_API_KEY=<your-key-optional>

# Disable Stripe for self-hosted (no payments)
DISABLE_STRIPE=true

# Optional: Scraping
ENABLE_BANDCAMP_SCRAPING=true
ENABLE_DISCOGS_API=true
DISCOGS_API_KEY=<your-key>
```

### Connecting Soul Player

In Soul Player settings:
```
Soul Services URL: https://your-domain.com
(or http://localhost:3001 for local dev)

Authentication: [ Configure OAuth ]
```

---

## Security Considerations

### API Key Storage (Self-Hosted)

User-provided API keys are encrypted at rest:
- **Algorithm**: AES-256-GCM
- **Key Derivation**: User's password (via Argon2)
- **Never logged**: Keys never appear in logs or errors

### Rate Limiting

Prevent abuse of external APIs:
- Per-user limits (tracked in `api_usage` table)
- Global rate limiter for external APIs
- Exponential backoff on failures

### Scraping Ethics

- Respect `robots.txt`
- User-Agent identifies as Soul Services
- Rate-limited (max 1 req/5 seconds per site)
- Cache aggressively (7-90 days)
- Fallback to APIs when available

### GDPR Compliance

- Users can delete all data (subscription history excluded for legal)
- No selling of data
- Optional analytics (user can opt-out)
- Self-hosted users have full control

---

## Open Questions / Future Considerations

### Short-Term

1. **Lyrics Legality**: Confirm licensing for Musixmatch/Genius. May need to use LRCLIB exclusively for non-commercial.
2. **AcoustID Costs**: At scale, fingerprinting could get expensive. Consider bulk pricing.
3. **MusicBrainz Rate Limits**: 1 req/sec is slow. May need to run local mirror for heavy users.

### Long-Term

4. **Multi-Tenant Self-Hosting**: Allow companies to host for multiple users (license required?)
5. **soul-sync Service**: Sync playlists/library across devices (separate service)
6. **soul-social Service**: Scrobbling, friend activity, shared playlists
7. **Mobile App Support**: Extend OAuth flow to iOS/Android clients
8. **Public API**: Let third-party apps use Soul Services (API key system)

---

## Success Metrics

### Technical

- [ ] 99.9% uptime (excluding maintenance)
- [ ] <200ms p95 latency for metadata endpoints
- [ ] <2s p95 latency for fingerprinting
- [ ] 80%+ cache hit rate for metadata

### Business

- [ ] 100 paying subscribers in first 3 months
- [ ] <5% monthly churn rate
- [ ] 70%+ gross margin (API costs < 30% revenue)
- [ ] Net Promoter Score (NPS) > 40

### Product

- [ ] 90%+ metadata match rate (MusicBrainz coverage)
- [ ] 80%+ lyrics match rate (Genius + LRCLIB)
- [ ] 95%+ fingerprint accuracy (AcoustID)
- [ ] <1% error rate on API calls

---

## FAQ

### For Users

**Q: Can I try before subscribing?**
A: We'll offer a 7-day free trial or limited free tier (TBD).

**Q: What if Soul Services shuts down?**
A: Soul Player works offline. Cached metadata remains. Self-hosted option always available.

**Q: Is my listening data shared?**
A: No. We only store metadata you request (artist names, track titles). No listening history unless you opt-in to scrobbling (future feature).

### For Developers

**Q: Will there be a public API?**
A: Phase 2 consideration. Initially only for Soul Player integration.

**Q: Can I contribute to Soul Services?**
A: The core will be closed-source initially, but we may open-source client SDKs and utilities.

**Q: What if I want to integrate into my own music app?**
A: Contact us for licensing. We're open to partnerships.

---

## Repository Structure (Proposed)

```
soul-services/
├── Cargo.toml                  # Workspace root
├── .env.example
├── docker-compose.yml
├── Dockerfile
│
├── crates/
│   ├── soul-discovery/         # Main service
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── api/            # Axum routes
│   │   │   ├── services/       # Business logic
│   │   │   ├── models/         # DB models
│   │   │   └── external/       # API clients
│   │   └── tests/
│   │
│   ├── soul-auth/              # OAuth + subscription logic
│   │   ├── src/
│   │   └── tests/
│   │
│   └── soul-services-client/   # Published SDK for Soul Player
│       ├── src/
│       └── examples/
│
├── migrations/                 # SQLx migrations
│   ├── 001_create_users.sql
│   ├── 002_create_subscriptions.sql
│   └── ...
│
├── docs/
│   ├── API.md                  # API documentation
│   ├── SELF_HOSTING.md
│   └── CONTRIBUTING.md
│
└── scripts/
    ├── setup.sh                # Local dev setup
    └── migrate.sh              # Run migrations
```

---

## Next Steps

1. **Create Repository**: `https://github.com/soulaudio/soul-services`
2. **Initial Commit**: Setup Cargo workspace + basic Axum server
3. **Deploy Fly.io**: Get production environment running early
4. **Stripe Dashboard**: Create products (Pro + Audiophile tiers)
5. **Domain Setup**: `services.soul.audio` + SSL certificate
6. **Begin Phase 1**: Follow implementation plan above

---

## Contact & Discussion

- **GitHub Issues**: For bugs, feature requests
- **Discussions**: Architecture decisions, community input
- **Email**: services@soul.audio (for partnerships, licensing)

---

**This is a living document. Update as architecture evolves.**
