# Multi-stage build for notes application
# Stage 1: Build the Rust application
FROM rust:1.75-slim-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock* ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only (cached unless Cargo.toml changes)
RUN cargo build --release && rm -rf src target/release/deps/notes*

# Copy actual source code
COPY src ./src

# Build the application
RUN cargo build --release

# Stage 2: Runtime image
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user
RUN groupadd -g 1000 notes && \
    useradd -u 1000 -g notes -s /bin/bash -m notes

WORKDIR /app

# Copy the binary from builder
COPY --from=builder /build/target/release/notes /app/notes

# Set ownership
RUN chown -R notes:notes /app

# Switch to non-root user
USER notes

# Expose port
EXPOSE 3000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://127.0.0.1:3000/ || exit 1

# Run the application
CMD ["/app/notes"]
