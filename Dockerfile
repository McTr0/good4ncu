# =============================================================================
# Good4NCU Backend Dockerfile
# Multi-stage build for Rust backend with minimal runtime image
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build
# -----------------------------------------------------------------------------
FROM rust:1.82-slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./

# Create dummy main.rs for dependency compilation
RUN mkdir -p src && echo "fn main() {}" > src/main.rs

# Build dependencies only (this layer will be cached)
RUN cargo build --release && rm -rf src

# Copy actual source code
COPY src ./src

# Build the application
RUN touch src/main.rs && cargo build --release

# -----------------------------------------------------------------------------
# Stage 2: Runtime
# -----------------------------------------------------------------------------
FROM gcr.io/distroless/cc-debian12 AS runtime

# Set labels
LABEL org.opencontainers.image.title="Good4NCU Backend"
LABEL org.opencontainers.image.description="Agentic secondhand marketplace for Chinese university campuses"
LABEL org.opencontainers.image.source="https://github.com/good4ncu/good4ncu"

# Copy the binary from builder
COPY --from=builder /app/target/release/good4ncu /usr/local/bin/good4ncu

# Create non-root user for security
RUN useradd --create-home --shell /bin/bash appuser && \
    chown -R appuser:appuser /home/appuser

USER appuser

# Set working directory
WORKDIR /home/appuser

# Expose port
EXPOSE 3000

# Set environment
ENV RUST_LOG=info
ENV RUST_BACKTRACE=1

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD wget --no-verbose --tries=1 --spider http://localhost:3000/api/health || exit 1

# Run the application
ENTRYPOINT ["good4ncu"]
