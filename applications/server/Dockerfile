# =============================================================================
# Stage 1: Build the web frontend
# =============================================================================
FROM node:20-alpine AS web-builder

WORKDIR /build

# Enable corepack for yarn
RUN corepack enable

# Copy workspace package files
COPY package.json yarn.lock .yarnrc.yml ./
COPY .yarn ./.yarn

# Copy relevant packages
COPY applications/web/package.json ./applications/web/
COPY applications/shared/package.json ./applications/shared/

# Install dependencies
RUN yarn install --immutable

# Copy source code
COPY applications/web ./applications/web
COPY applications/shared ./applications/shared

# Build the web app
WORKDIR /build/applications/web
RUN yarn build

# =============================================================================
# Stage 2: Build the Rust server
# =============================================================================
FROM rust:1.83-slim AS server-builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy workspace manifests
COPY Cargo.toml Cargo.lock ./

# Copy all crates (workspace structure)
COPY libraries ./libraries
COPY applications/server ./applications/server

# Build release binary
RUN cargo build --release -p soul-server

# =============================================================================
# Stage 3: Runtime image (server + web)
# =============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    ffmpeg \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 soul && \
    mkdir -p /app/data /app/data/tracks /app/web && \
    chown -R soul:soul /app

WORKDIR /app

# Copy server binary
COPY --from=server-builder /build/target/release/soul-server /usr/local/bin/soul-server

# Copy web frontend
COPY --from=web-builder /build/applications/web/dist /app/web

# Set web directory environment variable
ENV SOUL_WEB_DIR=/app/web

# Switch to non-root user
USER soul

# Expose server port
EXPOSE 8080

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/api/health || exit 1

# Default command
CMD ["soul-server", "serve"]
