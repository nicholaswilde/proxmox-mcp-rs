# Builder stage
FROM rust:latest as builder

WORKDIR /usr/src/app

# Copy manifests to cache dependencies
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && \
    echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy the source code
COPY src ./src

# Touch main.rs to force rebuild of the binary with new source
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM gcr.io/distroless/cc-debian12

WORKDIR /app

# Copy the binary from the builder
COPY --from=builder /usr/src/app/target/release/proxmox-mcp-rs /app/proxmox-mcp-rs

# Since we use rustls, we don't need OpenSSL libs, but we might need CA certs if not present (distroless usually has them)
# distroless/cc is good for standard Rust binaries.

ENTRYPOINT ["/app/proxmox-mcp-rs"]
