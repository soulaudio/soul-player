# Build stage
FROM rust:1.83-slim AS builder

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

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    ffmpeg \
    && rm -rf /var/lib/apt/lists/*

# Create app user
RUN useradd -m -u 1000 soul && \
    mkdir -p /app/data /app/data/tracks && \
    chown -R soul:soul /app

WORKDIR /app

# Copy binary from builder
COPY --from=builder /build/target/release/soul-server /usr/local/bin/soul-server

# Switch to non-root user
USER soul

# Expose server port
EXPOSE 8080

# Default command
CMD ["soul-server", "serve"]
