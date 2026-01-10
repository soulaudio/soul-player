# Soul Services - Architecture & Implementation Plan

**Status**: Planning Phase
**Target Implementation**: Separate Repository (`soul-services`)
**Stack**: Next.js 14+ (App Router) + PostgreSQL + Prisma
**Last Updated**: 2026-01-10

---

## Executive Summary

**Soul Services** is a separate, subscription-based platform that provides premium music metadata enrichment, discovery, and enhancement features for Soul Player and potentially other music applications.

### Why Separate from Soul Player?

1. **Business Model**: Soul Player is open-source; Soul Services is a sustainable revenue stream
2. **Scalability**: Independent scaling of web services
3. **Licensing**: Closed-source service protects business logic while keeping player open
4. **Infrastructure**: PostgreSQL + cloud hosting vs SQLite + local-first for player

### Why Next.js Instead of Rust?

Soul Services is a **web product** (API + dashboard + auth), not a performance-critical audio engine:

| Aspect | Soul Player | Soul Services |
|--------|-------------|---------------|
| **Type** | Desktop app + audio engine | Web API + dashboard |
| **CPU-intensive?** | Yes (audio decoding, DSP) | No (proxying APIs, caching) |
| **Best tool** | Rust | TypeScript/Next.js |

**Benefits of Next.js**:
- Faster development iteration
- Excellent auth ecosystem (NextAuth.js)
- Dashboard is just another route (no separate frontend)
- Serverless-ready (Vercel) or Docker deployable
- TypeScript end-to-end

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
│   Soul Player   │ (Open Source - Rust/Tauri)
│   (Desktop)     │
└────────┬────────┘
         │ OAuth 2.0 + PKCE
         │ REST API
         ▼
┌─────────────────────────────────────────────────────────────┐
│              Soul Services (Next.js)                         │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                   Next.js App                         │   │
│  │                                                       │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │   │
│  │  │  Dashboard  │  │  API Routes │  │    Auth     │  │   │
│  │  │   (React)   │  │  /api/v1/*  │  │ (NextAuth)  │  │   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │   │
│  │                                                       │   │
│  │  ┌─────────────────────────────────────────────────┐ │   │
│  │  │              Service Layer                       │ │   │
│  │  │  - MusicBrainz client                           │ │   │
│  │  │  - AcoustID client                              │ │   │
│  │  │  - Genius/LRCLIB client                         │ │   │
│  │  │  - Stripe integration                           │ │   │
│  │  └─────────────────────────────────────────────────┘ │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  ┌─────────────────────────────────────────────────────┐   │
│  │              PostgreSQL (via Prisma)                  │   │
│  │  - Users & Sessions                                   │   │
│  │  - Subscriptions (Stripe)                            │   │
│  │  - Cached Metadata                                    │   │
│  │  - API Usage Tracking                                │   │
│  └─────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
         │
         │ External APIs
         ▼
┌─────────────────────────────────────────┐
│      External Services                  │
│  - MusicBrainz API                      │
│  - AcoustID API                         │
│  - Genius API (Lyrics)                  │
│  - LRCLIB (Synced Lyrics)               │
│  - Discogs API                          │
│  - Stripe (Payments)                    │
└─────────────────────────────────────────┘
```

### Domain Structure

| Domain | Purpose |
|--------|---------|
| `soul.audio` | Marketing site (in soul-player repo) |
| `services.soul.audio` | API endpoints + Dashboard |
| `services.soul.audio/api/v1/*` | REST API for Soul Player |
| `services.soul.audio/dashboard/*` | Web dashboard for users |

---

## Repository Structure

```
soul-services/
├── package.json
├── next.config.ts
├── tailwind.config.ts
├── .env.example
├── docker-compose.yml
├── Dockerfile
│
├── prisma/
│   ├── schema.prisma              # Database schema
│   └── migrations/                # Prisma migrations
│
├── app/                           # Next.js App Router
│   ├── layout.tsx                 # Root layout
│   ├── page.tsx                   # Landing/marketing page
│   │
│   ├── (auth)/                    # Auth pages (public)
│   │   ├── login/page.tsx
│   │   ├── register/page.tsx
│   │   └── verify-email/page.tsx
│   │
│   ├── (dashboard)/               # Dashboard (authenticated)
│   │   ├── layout.tsx             # Dashboard layout with sidebar
│   │   ├── page.tsx               # Dashboard home
│   │   ├── account/page.tsx       # Account settings
│   │   ├── subscription/page.tsx  # Subscription management
│   │   ├── api-keys/page.tsx      # Self-hosted API key management
│   │   ├── devices/page.tsx       # Connected devices
│   │   └── usage/page.tsx         # API usage stats
│   │
│   ├── api/                       # API Routes
│   │   ├── auth/
│   │   │   └── [...nextauth]/route.ts  # NextAuth.js
│   │   │
│   │   ├── v1/                    # Public API (for Soul Player)
│   │   │   ├── fingerprint/route.ts
│   │   │   ├── lyrics/route.ts
│   │   │   ├── artist/
│   │   │   │   └── [id]/
│   │   │   │       └── enrich/route.ts
│   │   │   ├── album/
│   │   │   │   └── [id]/
│   │   │   │       └── enrich/route.ts
│   │   │   ├── discover/
│   │   │   │   ├── similar-artists/route.ts
│   │   │   │   ├── new-releases/route.ts
│   │   │   │   └── genre/[name]/route.ts
│   │   │   └── subscription/route.ts
│   │   │
│   │   ├── oauth/                 # OAuth Provider endpoints
│   │   │   ├── authorize/route.ts
│   │   │   ├── token/route.ts
│   │   │   └── revoke/route.ts
│   │   │
│   │   └── webhooks/
│   │       └── stripe/route.ts    # Stripe webhooks
│   │
│   └── docs/                      # API documentation (optional)
│       └── [[...slug]]/page.tsx   # Nextra or similar
│
├── lib/                           # Shared utilities
│   ├── auth.ts                    # NextAuth configuration
│   ├── db.ts                      # Prisma client
│   ├── stripe.ts                  # Stripe helpers
│   ├── rate-limit.ts              # Rate limiting
│   │
│   ├── external/                  # External API clients
│   │   ├── musicbrainz.ts
│   │   ├── acoustid.ts
│   │   ├── genius.ts
│   │   ├── lrclib.ts
│   │   └── discogs.ts
│   │
│   └── services/                  # Business logic
│       ├── enrichment.ts
│       ├── lyrics.ts
│       ├── discovery.ts
│       └── fingerprint.ts
│
├── components/                    # React components
│   ├── ui/                        # shadcn/ui components
│   ├── dashboard/
│   │   ├── Sidebar.tsx
│   │   ├── UsageChart.tsx
│   │   └── DeviceList.tsx
│   └── marketing/
│       ├── PricingTable.tsx
│       └── FeatureGrid.tsx
│
├── types/                         # TypeScript types
│   ├── api.ts
│   ├── musicbrainz.ts
│   └── index.ts
│
└── scripts/
    └── seed.ts                    # Database seeding
```

---

## Technology Stack

### Core Stack

| Component | Technology | Reasoning |
|-----------|------------|-----------|
| **Framework** | Next.js 14+ (App Router) | Full-stack React, API routes, SSR |
| **Language** | TypeScript | Type safety, great DX |
| **Database** | PostgreSQL 16 | JSON support, robust, scalable |
| **ORM** | Prisma | Type-safe queries, migrations, great DX |
| **Auth** | NextAuth.js v5 | OAuth providers, sessions, JWT |
| **Styling** | Tailwind CSS + shadcn/ui | Rapid UI development |
| **Validation** | Zod | Runtime type checking for API inputs |
| **HTTP Client** | Native fetch | Built into Next.js, no extra deps |

### External Services

| Service | Purpose | Cost Model |
|---------|---------|------------|
| **MusicBrainz** | Artist/album metadata | Free (1 req/sec, rate-limited) |
| **AcoustID** | Audio fingerprinting | $0.001/lookup (paid credits) |
| **Genius** | Lyrics (official) | Free API (rate-limited) |
| **LRCLIB** | Synced lyrics | Free (community-driven) |
| **Discogs** | Vinyl/physical data | Free API (rate-limited) |
| **Stripe** | Payment processing | 2.9% + $0.30/transaction |

### Infrastructure

| Component | Recommendation | Alternative |
|-----------|----------------|-------------|
| **Hosting** | Vercel (simplest) | Fly.io, Railway, Docker |
| **Database** | Vercel Postgres or Supabase | Fly Postgres, Neon |
| **Object Storage** | Cloudflare R2 | S3 |
| **CDN** | Vercel Edge / Cloudflare | - |
| **Monitoring** | Vercel Analytics + Sentry | - |
| **Background Jobs** | Vercel Cron or Inngest | BullMQ + Redis |

---

## Database Schema (Prisma)

```prisma
// prisma/schema.prisma

generator client {
  provider = "prisma-client-js"
}

datasource db {
  provider = "postgresql"
  url      = env("DATABASE_URL")
}

// ============ Users & Auth ============

model User {
  id            String    @id @default(cuid())
  email         String    @unique
  username      String    @unique
  passwordHash  String?   // null if using OAuth provider
  emailVerified DateTime?
  image         String?
  createdAt     DateTime  @default(now())
  updatedAt     DateTime  @updatedAt

  // Relations
  accounts      Account[]
  sessions      Session[]
  subscription  Subscription?
  apiKeys       ApiKey[]
  devices       Device[]
  apiUsage      ApiUsage[]
}

model Account {
  id                String  @id @default(cuid())
  userId            String
  type              String
  provider          String
  providerAccountId String
  refresh_token     String? @db.Text
  access_token      String? @db.Text
  expires_at        Int?
  token_type        String?
  scope             String?
  id_token          String? @db.Text
  session_state     String?

  user User @relation(fields: [userId], references: [id], onDelete: Cascade)

  @@unique([provider, providerAccountId])
}

model Session {
  id           String   @id @default(cuid())
  sessionToken String   @unique
  userId       String
  expires      DateTime

  user User @relation(fields: [userId], references: [id], onDelete: Cascade)
}

model VerificationToken {
  identifier String
  token      String   @unique
  expires    DateTime

  @@unique([identifier, token])
}

// ============ Subscriptions ============

model Subscription {
  id                   String   @id @default(cuid())
  userId               String   @unique
  stripeCustomerId     String   @unique
  stripeSubscriptionId String?  @unique
  tier                 Tier     @default(FREE)
  status               SubscriptionStatus @default(ACTIVE)
  currentPeriodEnd     DateTime?
  cancelAtPeriodEnd    Boolean  @default(false)
  createdAt            DateTime @default(now())
  updatedAt            DateTime @updatedAt

  user User @relation(fields: [userId], references: [id], onDelete: Cascade)
}

enum Tier {
  FREE
  PRO
  AUDIOPHILE
  SELF_HOSTED
}

enum SubscriptionStatus {
  ACTIVE
  CANCELED
  PAST_DUE
  TRIALING
}

// ============ Self-Hosted API Keys ============

model ApiKey {
  id            String   @id @default(cuid())
  userId        String
  service       String   // "musicbrainz", "acoustid", "genius"
  encryptedKey  String   // AES-256 encrypted
  isValid       Boolean  @default(true)
  lastCheckedAt DateTime?
  createdAt     DateTime @default(now())

  user User @relation(fields: [userId], references: [id], onDelete: Cascade)

  @@unique([userId, service])
}

// ============ Devices (OAuth Clients) ============

model Device {
  id         String     @id @default(cuid())
  userId     String
  name       String     // "John's MacBook Pro"
  type       DeviceType
  lastUsedAt DateTime?
  createdAt  DateTime   @default(now())

  // OAuth tokens for this device
  accessToken  String?  @unique
  refreshToken String?  @unique
  tokenExpiry  DateTime?

  user User @relation(fields: [userId], references: [id], onDelete: Cascade)

  @@unique([userId, name])
}

enum DeviceType {
  DESKTOP
  MOBILE
  WEB
  DAP
}

// ============ Usage Tracking ============

model ApiUsage {
  id           String   @id @default(cuid())
  userId       String
  endpoint     String
  requestCount Int      @default(1)
  date         DateTime @db.Date
  createdAt    DateTime @default(now())

  user User @relation(fields: [userId], references: [id], onDelete: Cascade)

  @@unique([userId, endpoint, date])
  @@index([userId, date])
}

// ============ Metadata Cache ============

model ArtistMetadata {
  id             String   @id @default(cuid())
  musicbrainzId  String   @unique
  name           String
  sortName       String?
  bio            String?  @db.Text
  country        String?
  formedDate     DateTime?
  genreTags      Json?    // ["rock", "progressive rock"]
  similarArtists Json?    // [{"id": "...", "name": "..."}, ...]
  cachedAt       DateTime @default(now())
  expiresAt      DateTime

  @@index([musicbrainzId])
}

model AlbumMetadata {
  id              String   @id @default(cuid())
  musicbrainzId   String   @unique
  title           String
  artistMbId      String?
  releaseDate     DateTime?
  label           String?
  catalogNumber   String?
  coverArtUrl     String?
  coverArtHiresUrl String?
  trackListing    Json?
  cachedAt        DateTime @default(now())
  expiresAt       DateTime

  @@index([musicbrainzId])
}

model Lyrics {
  id               String   @id @default(cuid())
  trackFingerprint String   // SHA-256 of artist+title+album
  artist           String
  title            String
  album            String?
  lyrics           String   @db.Text
  synced           Boolean  @default(false)
  language         String   @default("en")
  source           String   // "genius", "lrclib", "musixmatch"
  cachedAt         DateTime @default(now())
  expiresAt        DateTime

  @@unique([trackFingerprint])
  @@index([artist, title])
}

model Fingerprint {
  id                 String   @id @default(cuid())
  userId             String
  fingerprint        String   @db.Text
  duration           Int
  matchedRecordingId String?  // MusicBrainz recording ID
  matchScore         Float?
  createdAt          DateTime @default(now())

  @@index([userId, createdAt])
}
```

---

## Authentication

### NextAuth.js Configuration

```typescript
// lib/auth.ts
import NextAuth from "next-auth"
import { PrismaAdapter } from "@auth/prisma-adapter"
import Credentials from "next-auth/providers/credentials"
import Google from "next-auth/providers/google"
import { prisma } from "./db"
import { compare } from "bcryptjs"

export const { handlers, auth, signIn, signOut } = NextAuth({
  adapter: PrismaAdapter(prisma),
  session: { strategy: "jwt" },

  providers: [
    // Email/password login
    Credentials({
      credentials: {
        email: { label: "Email", type: "email" },
        password: { label: "Password", type: "password" }
      },
      async authorize(credentials) {
        if (!credentials?.email || !credentials?.password) return null

        const user = await prisma.user.findUnique({
          where: { email: credentials.email as string },
          include: { subscription: true }
        })

        if (!user?.passwordHash) return null

        const isValid = await compare(
          credentials.password as string,
          user.passwordHash
        )
        if (!isValid) return null

        return {
          id: user.id,
          email: user.email,
          name: user.username,
          tier: user.subscription?.tier ?? "FREE"
        }
      }
    }),

    // Optional: OAuth providers
    Google({
      clientId: process.env.GOOGLE_CLIENT_ID!,
      clientSecret: process.env.GOOGLE_CLIENT_SECRET!,
    }),
  ],

  callbacks: {
    async jwt({ token, user }) {
      if (user) {
        token.id = user.id
        token.tier = (user as any).tier
      }
      return token
    },
    async session({ session, token }) {
      session.user.id = token.id as string
      session.user.tier = token.tier as string
      return session
    }
  },

  pages: {
    signIn: "/login",
    error: "/login",
  }
})
```

### OAuth 2.0 + PKCE for Soul Player

Soul Player (desktop app) authenticates via OAuth with PKCE:

```typescript
// app/api/oauth/authorize/route.ts
import { NextRequest, NextResponse } from "next/server"
import { prisma } from "@/lib/db"
import { auth } from "@/lib/auth"

export async function GET(req: NextRequest) {
  const session = await auth()
  const { searchParams } = new URL(req.url)

  const clientId = searchParams.get("client_id")
  const redirectUri = searchParams.get("redirect_uri") // soul://oauth/callback
  const codeChallenge = searchParams.get("code_challenge")
  const codeChallengeMethod = searchParams.get("code_challenge_method") // S256
  const state = searchParams.get("state")

  // Validate params
  if (!codeChallenge || codeChallengeMethod !== "S256") {
    return NextResponse.json({ error: "PKCE required" }, { status: 400 })
  }

  // If not logged in, redirect to login with return URL
  if (!session?.user) {
    const returnUrl = encodeURIComponent(req.url)
    return NextResponse.redirect(new URL(`/login?returnTo=${returnUrl}`, req.url))
  }

  // Generate authorization code
  const code = crypto.randomUUID()

  // Store code + challenge (5 min expiry)
  await prisma.authorizationCode.create({
    data: {
      code,
      userId: session.user.id,
      codeChallenge,
      redirectUri,
      expiresAt: new Date(Date.now() + 5 * 60 * 1000)
    }
  })

  // Redirect back to Soul Player
  const callbackUrl = new URL(redirectUri!)
  callbackUrl.searchParams.set("code", code)
  if (state) callbackUrl.searchParams.set("state", state)

  return NextResponse.redirect(callbackUrl)
}
```

```typescript
// app/api/oauth/token/route.ts
import { NextRequest, NextResponse } from "next/server"
import { prisma } from "@/lib/db"
import { createHash } from "crypto"
import { SignJWT } from "jose"

export async function POST(req: NextRequest) {
  const body = await req.json()
  const { code, code_verifier, grant_type } = body

  if (grant_type === "authorization_code") {
    // Find the authorization code
    const authCode = await prisma.authorizationCode.findUnique({
      where: { code },
      include: { user: { include: { subscription: true } } }
    })

    if (!authCode || authCode.expiresAt < new Date()) {
      return NextResponse.json({ error: "invalid_grant" }, { status: 400 })
    }

    // Verify PKCE
    const expectedChallenge = createHash("sha256")
      .update(code_verifier)
      .digest("base64url")

    if (expectedChallenge !== authCode.codeChallenge) {
      return NextResponse.json({ error: "invalid_grant" }, { status: 400 })
    }

    // Generate tokens
    const accessToken = await new SignJWT({
      sub: authCode.user.id,
      tier: authCode.user.subscription?.tier ?? "FREE"
    })
      .setProtectedHeader({ alg: "HS256" })
      .setExpirationTime("1h")
      .sign(new TextEncoder().encode(process.env.JWT_SECRET))

    const refreshToken = crypto.randomUUID()

    // Store refresh token
    await prisma.device.upsert({
      where: { accessToken },
      create: {
        userId: authCode.user.id,
        name: body.device_name ?? "Unknown Device",
        type: body.device_type ?? "DESKTOP",
        accessToken,
        refreshToken,
        tokenExpiry: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000) // 30 days
      },
      update: {
        accessToken,
        refreshToken,
        tokenExpiry: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000)
      }
    })

    // Delete used auth code
    await prisma.authorizationCode.delete({ where: { code } })

    return NextResponse.json({
      access_token: accessToken,
      refresh_token: refreshToken,
      token_type: "Bearer",
      expires_in: 3600
    })
  }

  if (grant_type === "refresh_token") {
    // Handle refresh token flow
    // ...
  }

  return NextResponse.json({ error: "unsupported_grant_type" }, { status: 400 })
}
```

---

## API Implementation Examples

### Lyrics Endpoint

```typescript
// app/api/v1/lyrics/route.ts
import { NextRequest, NextResponse } from "next/server"
import { auth } from "@/lib/auth"
import { prisma } from "@/lib/db"
import { getLyricsFromGenius } from "@/lib/external/genius"
import { getLyricsFromLrclib } from "@/lib/external/lrclib"
import { createHash } from "crypto"
import { z } from "zod"

const querySchema = z.object({
  artist: z.string().min(1),
  title: z.string().min(1),
  album: z.string().optional(),
  duration: z.coerce.number().optional(),
  synced: z.coerce.boolean().default(false)
})

export async function GET(req: NextRequest) {
  // Authenticate
  const session = await auth()
  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 })
  }

  // Parse & validate query params
  const { searchParams } = new URL(req.url)
  const parsed = querySchema.safeParse(Object.fromEntries(searchParams))

  if (!parsed.success) {
    return NextResponse.json({
      error: "Invalid parameters",
      details: parsed.error.flatten()
    }, { status: 400 })
  }

  const { artist, title, album, duration, synced } = parsed.data

  // Check rate limit
  const tier = session.user.tier
  const rateLimit = tier === "AUDIOPHILE" ? 120 : 60
  // ... rate limiting logic

  // Check cache first
  const fingerprint = createHash("sha256")
    .update(`${artist.toLowerCase()}|${title.toLowerCase()}|${album?.toLowerCase() ?? ""}`)
    .digest("hex")

  const cached = await prisma.lyrics.findUnique({
    where: { trackFingerprint: fingerprint }
  })

  if (cached && cached.expiresAt > new Date()) {
    // Return cached if synced requirement is met
    if (!synced || cached.synced) {
      return NextResponse.json({
        lyrics: cached.lyrics,
        synced: cached.synced,
        source: cached.source,
        language: cached.language,
        cached: true
      })
    }
  }

  // Fetch from external sources
  let lyrics = null
  let source = ""
  let isSynced = false

  // Try LRCLIB first for synced lyrics
  if (synced) {
    const lrcResult = await getLyricsFromLrclib(artist, title, album, duration)
    if (lrcResult?.syncedLyrics) {
      lyrics = lrcResult.syncedLyrics
      source = "lrclib"
      isSynced = true
    }
  }

  // Fallback to Genius for unsynced
  if (!lyrics) {
    const geniusResult = await getLyricsFromGenius(artist, title)
    if (geniusResult) {
      lyrics = geniusResult.lyrics
      source = "genius"
      isSynced = false
    }
  }

  if (!lyrics) {
    return NextResponse.json({ error: "Lyrics not found" }, { status: 404 })
  }

  // Cache the result
  await prisma.lyrics.upsert({
    where: { trackFingerprint: fingerprint },
    create: {
      trackFingerprint: fingerprint,
      artist,
      title,
      album,
      lyrics,
      synced: isSynced,
      source,
      expiresAt: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000) // 30 days
    },
    update: {
      lyrics,
      synced: isSynced,
      source,
      expiresAt: new Date(Date.now() + 30 * 24 * 60 * 60 * 1000)
    }
  })

  // Track usage
  await trackApiUsage(session.user.id, "lyrics")

  return NextResponse.json({
    lyrics,
    synced: isSynced,
    source,
    language: "en",
    cached: false
  })
}

async function trackApiUsage(userId: string, endpoint: string) {
  const today = new Date()
  today.setHours(0, 0, 0, 0)

  await prisma.apiUsage.upsert({
    where: {
      userId_endpoint_date: { userId, endpoint, date: today }
    },
    create: { userId, endpoint, date: today, requestCount: 1 },
    update: { requestCount: { increment: 1 } }
  })
}
```

### Artist Enrichment Endpoint

```typescript
// app/api/v1/artist/[id]/enrich/route.ts
import { NextRequest, NextResponse } from "next/server"
import { auth } from "@/lib/auth"
import { prisma } from "@/lib/db"
import { getArtistFromMusicBrainz } from "@/lib/external/musicbrainz"

export async function GET(
  req: NextRequest,
  { params }: { params: { id: string } }
) {
  const session = await auth()
  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 })
  }

  const musicbrainzId = params.id

  // Check cache
  const cached = await prisma.artistMetadata.findUnique({
    where: { musicbrainzId }
  })

  if (cached && cached.expiresAt > new Date()) {
    return NextResponse.json({
      ...cached,
      cached: true
    })
  }

  // Fetch from MusicBrainz
  const artist = await getArtistFromMusicBrainz(musicbrainzId)

  if (!artist) {
    return NextResponse.json({ error: "Artist not found" }, { status: 404 })
  }

  // Cache for 90 days (older artists rarely change)
  const expiresAt = new Date(Date.now() + 90 * 24 * 60 * 60 * 1000)

  await prisma.artistMetadata.upsert({
    where: { musicbrainzId },
    create: {
      musicbrainzId,
      name: artist.name,
      sortName: artist.sortName,
      bio: artist.bio,
      country: artist.country,
      formedDate: artist.formedDate,
      genreTags: artist.genreTags,
      similarArtists: artist.similarArtists,
      expiresAt
    },
    update: {
      name: artist.name,
      sortName: artist.sortName,
      bio: artist.bio,
      country: artist.country,
      formedDate: artist.formedDate,
      genreTags: artist.genreTags,
      similarArtists: artist.similarArtists,
      expiresAt
    }
  })

  await trackApiUsage(session.user.id, "artist_enrich")

  return NextResponse.json({
    ...artist,
    cached: false
  })
}
```

### Fingerprint Endpoint (Audiophile Tier)

```typescript
// app/api/v1/fingerprint/route.ts
import { NextRequest, NextResponse } from "next/server"
import { auth } from "@/lib/auth"
import { prisma } from "@/lib/db"
import { lookupFingerprint } from "@/lib/external/acoustid"
import { getArtistFromMusicBrainz } from "@/lib/external/musicbrainz"
import { z } from "zod"

const bodySchema = z.object({
  fingerprint: z.string().min(1),
  duration: z.number().int().positive(),
  enrich: z.boolean().default(false)
})

export async function POST(req: NextRequest) {
  const session = await auth()
  if (!session?.user) {
    return NextResponse.json({ error: "Unauthorized" }, { status: 401 })
  }

  // Check tier - fingerprinting is Audiophile only
  if (session.user.tier !== "AUDIOPHILE" && session.user.tier !== "SELF_HOSTED") {
    return NextResponse.json({
      error: "Fingerprinting requires Audiophile tier",
      upgrade_url: "/dashboard/subscription"
    }, { status: 403 })
  }

  // Check monthly quota (1000 for Audiophile)
  const startOfMonth = new Date()
  startOfMonth.setDate(1)
  startOfMonth.setHours(0, 0, 0, 0)

  const usage = await prisma.apiUsage.aggregate({
    where: {
      userId: session.user.id,
      endpoint: "fingerprint",
      date: { gte: startOfMonth }
    },
    _sum: { requestCount: true }
  })

  const monthlyUsage = usage._sum.requestCount ?? 0
  if (session.user.tier === "AUDIOPHILE" && monthlyUsage >= 1000) {
    return NextResponse.json({
      error: {
        code: "RATE_LIMIT_EXCEEDED",
        message: "Monthly fingerprint quota exceeded (1000)",
        details: {
          current_usage: monthlyUsage,
          limit: 1000,
          reset_at: new Date(startOfMonth.getFullYear(), startOfMonth.getMonth() + 1, 1)
        }
      }
    }, { status: 429 })
  }

  const body = await req.json()
  const parsed = bodySchema.safeParse(body)

  if (!parsed.success) {
    return NextResponse.json({ error: parsed.error.flatten() }, { status: 400 })
  }

  const { fingerprint, duration, enrich } = parsed.data

  // Lookup via AcoustID
  const result = await lookupFingerprint(fingerprint, duration)

  if (!result || result.results.length === 0) {
    return NextResponse.json({
      match_score: 0,
      track: null,
      message: "No match found"
    })
  }

  const bestMatch = result.results[0]
  const recording = bestMatch.recordings?.[0]

  // Store for analytics
  await prisma.fingerprint.create({
    data: {
      userId: session.user.id,
      fingerprint: fingerprint.substring(0, 100), // Truncate for storage
      duration,
      matchedRecordingId: recording?.id,
      matchScore: bestMatch.score
    }
  })

  await trackApiUsage(session.user.id, "fingerprint")

  let enrichedData = null
  if (enrich && recording?.artists?.[0]?.id) {
    enrichedData = await getArtistFromMusicBrainz(recording.artists[0].id)
  }

  return NextResponse.json({
    match_score: bestMatch.score,
    track: recording ? {
      musicbrainz_id: recording.id,
      title: recording.title,
      artist: recording.artists?.[0]?.name,
      album: recording.releasegroups?.[0]?.title,
    } : null,
    enriched: enrichedData
  })
}
```

---

## Dashboard Pages

### Subscription Management

```typescript
// app/(dashboard)/subscription/page.tsx
import { auth } from "@/lib/auth"
import { prisma } from "@/lib/db"
import { redirect } from "next/navigation"
import { SubscriptionCard } from "@/components/dashboard/SubscriptionCard"
import { PricingTable } from "@/components/marketing/PricingTable"
import { createCheckoutSession, createPortalSession } from "@/lib/stripe"

export default async function SubscriptionPage() {
  const session = await auth()
  if (!session?.user) redirect("/login")

  const subscription = await prisma.subscription.findUnique({
    where: { userId: session.user.id }
  })

  // Get usage stats
  const startOfMonth = new Date()
  startOfMonth.setDate(1)
  startOfMonth.setHours(0, 0, 0, 0)

  const usage = await prisma.apiUsage.groupBy({
    by: ["endpoint"],
    where: {
      userId: session.user.id,
      date: { gte: startOfMonth }
    },
    _sum: { requestCount: true }
  })

  return (
    <div className="space-y-8">
      <h1 className="text-3xl font-bold">Subscription</h1>

      {subscription ? (
        <SubscriptionCard
          subscription={subscription}
          usage={usage}
          onManage={async () => {
            "use server"
            const url = await createPortalSession(subscription.stripeCustomerId)
            redirect(url)
          }}
        />
      ) : (
        <div className="space-y-6">
          <p className="text-muted-foreground">
            Upgrade to unlock premium features like lyrics, metadata enrichment,
            and audio fingerprinting.
          </p>
          <PricingTable
            onSelectPlan={async (tier: string) => {
              "use server"
              const url = await createCheckoutSession(session.user.id, tier)
              redirect(url)
            }}
          />
        </div>
      )}
    </div>
  )
}
```

### API Keys Management (Self-Hosted)

```typescript
// app/(dashboard)/api-keys/page.tsx
import { auth } from "@/lib/auth"
import { prisma } from "@/lib/db"
import { redirect } from "next/navigation"
import { ApiKeyForm } from "@/components/dashboard/ApiKeyForm"
import { ApiKeyList } from "@/components/dashboard/ApiKeyList"

const SERVICES = [
  { id: "musicbrainz", name: "MusicBrainz", required: false, freeApi: true },
  { id: "acoustid", name: "AcoustID", required: false, freeApi: false },
  { id: "genius", name: "Genius", required: false, freeApi: true },
  { id: "discogs", name: "Discogs", required: false, freeApi: true },
]

export default async function ApiKeysPage() {
  const session = await auth()
  if (!session?.user) redirect("/login")

  // Only show for self-hosted tier
  const subscription = await prisma.subscription.findUnique({
    where: { userId: session.user.id }
  })

  if (subscription?.tier !== "SELF_HOSTED") {
    return (
      <div className="space-y-4">
        <h1 className="text-3xl font-bold">API Keys</h1>
        <p className="text-muted-foreground">
          API key management is only available for Self-Hosted tier users.
          With Pro or Audiophile tiers, we handle all API integrations for you.
        </p>
      </div>
    )
  }

  const apiKeys = await prisma.apiKey.findMany({
    where: { userId: session.user.id },
    select: {
      id: true,
      service: true,
      isValid: true,
      lastCheckedAt: true,
      createdAt: true
      // Note: we never return the actual key
    }
  })

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold">API Keys</h1>
        <p className="text-muted-foreground mt-2">
          Configure your own API keys for external services.
          Keys are encrypted at rest.
        </p>
      </div>

      <ApiKeyList keys={apiKeys} services={SERVICES} />

      <ApiKeyForm services={SERVICES} existingKeys={apiKeys} />
    </div>
  )
}
```

---

## Integration with Soul Player

### Rust Client SDK

The Soul Player desktop app (Rust/Tauri) uses a client library to communicate with Soul Services:

```rust
// In soul-player repo: libraries/soul-services-client/src/lib.rs

use reqwest::Client;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SoulServicesError {
    #[error("Not authenticated - please sign in")]
    Unauthenticated,
    #[error("Subscription required: {0}")]
    SubscriptionRequired(String),
    #[error("Rate limit exceeded")]
    RateLimited,
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Not found")]
    NotFound,
}

pub struct SoulServicesClient {
    base_url: String,
    client: Client,
    access_token: Option<String>,
}

impl SoulServicesClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: Client::new(),
            access_token: None,
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.access_token = Some(token);
        self
    }

    pub async fn get_lyrics(
        &self,
        artist: &str,
        title: &str,
        synced: bool,
    ) -> Result<LyricsResponse, SoulServicesError> {
        let token = self.access_token.as_ref()
            .ok_or(SoulServicesError::Unauthenticated)?;

        let resp = self.client
            .get(format!("{}/api/v1/lyrics", self.base_url))
            .query(&[
                ("artist", artist),
                ("title", title),
                ("synced", if synced { "true" } else { "false" }),
            ])
            .bearer_auth(token)
            .send()
            .await?;

        match resp.status().as_u16() {
            200 => Ok(resp.json().await?),
            401 => Err(SoulServicesError::Unauthenticated),
            403 => Err(SoulServicesError::SubscriptionRequired(
                "Lyrics require Pro tier".into()
            )),
            404 => Err(SoulServicesError::NotFound),
            429 => Err(SoulServicesError::RateLimited),
            _ => Err(SoulServicesError::Network(
                resp.error_for_status().unwrap_err()
            )),
        }
    }

    pub async fn enrich_artist(
        &self,
        musicbrainz_id: &str,
    ) -> Result<ArtistMetadata, SoulServicesError> {
        let token = self.access_token.as_ref()
            .ok_or(SoulServicesError::Unauthenticated)?;

        let resp = self.client
            .get(format!("{}/api/v1/artist/{}/enrich", self.base_url, musicbrainz_id))
            .bearer_auth(token)
            .send()
            .await?;

        match resp.status().as_u16() {
            200 => Ok(resp.json().await?),
            401 => Err(SoulServicesError::Unauthenticated),
            404 => Err(SoulServicesError::NotFound),
            429 => Err(SoulServicesError::RateLimited),
            _ => Err(SoulServicesError::Network(
                resp.error_for_status().unwrap_err()
            )),
        }
    }

    pub async fn fingerprint(
        &self,
        fingerprint: &str,
        duration: u32,
        enrich: bool,
    ) -> Result<FingerprintResponse, SoulServicesError> {
        let token = self.access_token.as_ref()
            .ok_or(SoulServicesError::Unauthenticated)?;

        let resp = self.client
            .post(format!("{}/api/v1/fingerprint", self.base_url))
            .bearer_auth(token)
            .json(&serde_json::json!({
                "fingerprint": fingerprint,
                "duration": duration,
                "enrich": enrich
            }))
            .send()
            .await?;

        match resp.status().as_u16() {
            200 => Ok(resp.json().await?),
            401 => Err(SoulServicesError::Unauthenticated),
            403 => Err(SoulServicesError::SubscriptionRequired(
                "Fingerprinting requires Audiophile tier".into()
            )),
            429 => Err(SoulServicesError::RateLimited),
            _ => Err(SoulServicesError::Network(
                resp.error_for_status().unwrap_err()
            )),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LyricsResponse {
    pub lyrics: String,
    pub synced: bool,
    pub source: String,
    pub language: String,
}

#[derive(Debug, Deserialize)]
pub struct ArtistMetadata {
    pub musicbrainz_id: String,
    pub name: String,
    pub bio: Option<String>,
    pub country: Option<String>,
    pub formed_date: Option<String>,
    pub genre_tags: Option<Vec<String>>,
    pub similar_artists: Option<Vec<SimilarArtist>>,
}

#[derive(Debug, Deserialize)]
pub struct SimilarArtist {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct FingerprintResponse {
    pub match_score: f32,
    pub track: Option<TrackMatch>,
}

#[derive(Debug, Deserialize)]
pub struct TrackMatch {
    pub musicbrainz_id: String,
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
}
```

---

## Deployment

### Vercel (Recommended for Simplicity)

```bash
# Install Vercel CLI
npm i -g vercel

# Deploy
vercel

# Set environment variables in Vercel dashboard:
# DATABASE_URL, NEXTAUTH_SECRET, STRIPE_SECRET_KEY, etc.
```

### Docker (Self-Hosted)

```dockerfile
# Dockerfile
FROM node:20-alpine AS base

FROM base AS deps
WORKDIR /app
COPY package.json yarn.lock* ./
RUN yarn install --frozen-lockfile

FROM base AS builder
WORKDIR /app
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN npx prisma generate
RUN yarn build

FROM base AS runner
WORKDIR /app
ENV NODE_ENV=production

RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs

COPY --from=builder /app/public ./public
COPY --from=builder --chown=nextjs:nodejs /app/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /app/.next/static ./.next/static
COPY --from=builder /app/prisma ./prisma

USER nextjs
EXPOSE 3000
ENV PORT 3000

CMD ["node", "server.js"]
```

```yaml
# docker-compose.yml
version: '3.8'
services:
  soul-services:
    build: .
    ports:
      - "3000:3000"
    environment:
      DATABASE_URL: postgres://user:pass@db:5432/soul_services
      NEXTAUTH_URL: http://localhost:3000
      NEXTAUTH_SECRET: ${NEXTAUTH_SECRET}
      STRIPE_SECRET_KEY: ${STRIPE_SECRET_KEY}
      STRIPE_WEBHOOK_SECRET: ${STRIPE_WEBHOOK_SECRET}
    depends_on:
      - db

  db:
    image: postgres:16
    volumes:
      - postgres_data:/var/lib/postgresql/data
    environment:
      POSTGRES_DB: soul_services
      POSTGRES_USER: user
      POSTGRES_PASSWORD: pass
    ports:
      - "5432:5432"

volumes:
  postgres_data:
```

### Environment Variables

```bash
# .env.example

# Database
DATABASE_URL="postgresql://user:password@localhost:5432/soul_services"

# NextAuth
NEXTAUTH_URL="http://localhost:3000"
NEXTAUTH_SECRET="generate-with-openssl-rand-base64-32"

# OAuth Providers (optional)
GOOGLE_CLIENT_ID=""
GOOGLE_CLIENT_SECRET=""

# Stripe
STRIPE_SECRET_KEY="sk_test_..."
STRIPE_WEBHOOK_SECRET="whsec_..."
STRIPE_PRICE_PRO="price_..."
STRIPE_PRICE_AUDIOPHILE="price_..."

# External APIs
MUSICBRAINZ_APP_NAME="SoulServices"
MUSICBRAINZ_APP_VERSION="1.0.0"
MUSICBRAINZ_CONTACT="contact@soul.audio"

ACOUSTID_API_KEY=""
GENIUS_ACCESS_TOKEN=""
DISCOGS_API_KEY=""

# Self-hosted mode (disables Stripe)
SELF_HOSTED_MODE="false"
```

---

## Implementation Phases

### Phase 1: Foundation (2-3 weeks)

- [ ] Repository setup with Next.js 14 App Router
- [ ] Prisma schema + PostgreSQL setup
- [ ] NextAuth.js configuration (credentials + optional OAuth)
- [ ] OAuth 2.0 + PKCE endpoints for Soul Player
- [ ] Basic dashboard layout (account page)
- [ ] Docker + docker-compose for local dev
- [ ] Deploy to Vercel (or Fly.io)

### Phase 2: Core API (2-3 weeks)

- [ ] MusicBrainz client + artist/album enrichment
- [ ] Lyrics fetching (Genius + LRCLIB)
- [ ] Response caching with Prisma
- [ ] Rate limiting middleware
- [ ] API usage tracking
- [ ] Stripe subscription integration
- [ ] Subscription management dashboard page

### Phase 3: Advanced Features (2-3 weeks)

- [ ] AcoustID fingerprinting (Audiophile tier)
- [ ] Discovery endpoints (similar artists, new releases)
- [ ] Self-hosted API key management
- [ ] Usage statistics dashboard

### Phase 4: Polish (1-2 weeks)

- [ ] Rust client SDK (`soul-services-client` crate)
- [ ] API documentation (OpenAPI/Swagger or Nextra docs)
- [ ] Error handling improvements
- [ ] Sentry integration
- [ ] Self-hosting documentation

---

## Success Metrics

### Technical

- [ ] 99.9% uptime
- [ ] <200ms p95 latency for metadata endpoints
- [ ] <2s p95 latency for fingerprinting
- [ ] 80%+ cache hit rate

### Business

- [ ] 100 paying subscribers in first 3 months
- [ ] <5% monthly churn rate
- [ ] 70%+ gross margin

### Product

- [ ] 90%+ metadata match rate
- [ ] 80%+ lyrics match rate
- [ ] 95%+ fingerprint accuracy
- [ ] <1% API error rate

---

## Next Steps

1. **Create Repository**: `https://github.com/soulaudio/soul-services`
2. **Initial Setup**: `npx create-next-app@latest soul-services --typescript --tailwind --app`
3. **Add Prisma**: `npx prisma init`
4. **Deploy to Vercel**: Connect repo, add environment variables
5. **Stripe Dashboard**: Create products (Pro + Audiophile tiers)
6. **Domain Setup**: `services.soul.audio` pointing to Vercel

---

**This is a living document. Update as architecture evolves.**
