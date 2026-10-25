# syntax=docker/dockerfile:1

# Stage 1: Build binary
FROM rust:1.85-bookworm AS builder

WORKDIR /build

# Copy workspace sources and assets needed for compilation
COPY Cargo.toml Cargo.lock ./
COPY resources/ resources/
COPY crates/ crates/
COPY examples/ examples/

# Compile eidolon-server in release mode
RUN cargo build --release -p eidolon-server

# Stage 2: Minimal runtime image
FROM debian:bookworm-slim AS runner

# Install graphics drivers & runtime tools:
# - ca-certificates: HTTPS certificate verification for remote skin/cape fetching
# - libvulkan1 & mesa-vulkan-drivers: Vulkan loader & Lavapipe CPU rasterizer
# - libgl1-mesa-dri: OpenGL / llvmpipe fallback
# - curl: container healthcheck
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libvulkan1 \
    mesa-vulkan-drivers \
    libgl1-mesa-dri \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Default headless software rendering & server config
ENV LIBGL_ALWAYS_SOFTWARE=1 \
    MESA_LOADER_DRIVER_OVERRIDE=llvmpipe \
    EIDOLON_HOST=0.0.0.0 \
    EIDOLON_PORT=8080 \
    EIDOLON_WORKERS=2 \
    RUST_LOG=info

WORKDIR /app

COPY --from=builder /build/target/release/eidolon-server /usr/local/bin/eidolon-server

RUN useradd -m -u 1000 -U eidolon
USER eidolon

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://127.0.0.1:${EIDOLON_PORT}/health || exit 1

ENTRYPOINT ["eidolon-server"]
