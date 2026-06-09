FROM rust:1.81-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y -o Acquire::Retries=3 --no-install-recommends \
    pkg-config libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo.toml and source
COPY Cargo.toml .
COPY src/ src/

# Build with release profile
RUN cargo build --release

# Runtime stage
FROM debian:12-slim

WORKDIR /app

# Copy binary
COPY --from=builder /app/target/release/ai-gateway-rs /app/ai-gateway-rs

# Expose port
EXPOSE 2887

# Run
CMD ["/app/ai-gateway-rs"]
