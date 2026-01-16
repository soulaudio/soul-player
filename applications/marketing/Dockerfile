# =============================================================================
# Soul Player Marketing Site Dockerfile
# =============================================================================
# IMPORTANT: This Dockerfile MUST be run from the project root, not from applications/marketing/
#
# ✅ CORRECT:   docker build -f applications/marketing/Dockerfile -t soul-marketing .
# ❌ INCORRECT: cd applications/marketing && docker build -t soul-marketing .
#
# The build context must be the project root to access:
#   - package.json, yarn.lock (workspace root)
#   - applications/shared (shared TypeScript code)
#   - applications/marketing (marketing site source)
# =============================================================================

# Stage 1: Dependencies
FROM node:20-alpine AS deps
RUN apk add --no-cache libc6-compat

WORKDIR /build

# Enable corepack for yarn
RUN corepack enable

# Copy workspace package files
COPY package.json yarn.lock .yarnrc.yml ./

# Copy relevant packages
COPY applications/marketing/package.json ./applications/marketing/
COPY applications/shared/package.json ./applications/shared/

# Install dependencies
RUN yarn install --immutable

# Stage 2: Builder
FROM node:20-alpine AS builder
WORKDIR /build

# Enable corepack for yarn
RUN corepack enable

# Copy dependencies from deps stage
COPY --from=deps /build/node_modules ./node_modules
COPY --from=deps /build/.yarn ./.yarn
COPY --from=deps /build/.yarnrc.yml ./

# Copy workspace package files
COPY package.json yarn.lock ./

# Copy source code
COPY applications/marketing ./applications/marketing
COPY applications/shared ./applications/shared

# Set environment variables for build
ENV NEXT_TELEMETRY_DISABLED=1
ENV NODE_ENV=production

# Build the marketing application
WORKDIR /build/applications/marketing
RUN yarn build

# Stage 3: Runner
FROM node:20-alpine AS runner
WORKDIR /app

# Set environment
ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1

# Create non-root user
RUN addgroup --system --gid 1001 nodejs
RUN adduser --system --uid 1001 nextjs

# Copy built application
COPY --from=builder /build/applications/marketing/public ./public
COPY --from=builder --chown=nextjs:nodejs /build/applications/marketing/.next/standalone ./
COPY --from=builder --chown=nextjs:nodejs /build/applications/marketing/.next/static ./.next/static

USER nextjs

EXPOSE 3000

ENV PORT=3000
ENV HOSTNAME="0.0.0.0"

CMD ["node", "server.js"]
