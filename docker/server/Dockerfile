# Use the official Rust image as the base image
FROM rust:1.75-slim AS builder

# Set working directory
WORKDIR /app

# Copy the entire project
COPY . .

# Install build dependencies
RUN apt-get update && \
    apt-get install -y build-essential pkg-config libssl-dev

# Build the server in release mode
RUN cargo build --release --bin mcpi-server

# Create a slim runtime image
FROM debian:bullseye-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Set working directory
WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/mcpi-server /app/mcpi-server

# Copy data directory
COPY data /app/data

# Expose the default port
EXPOSE 3001

# Set the entrypoint
ENTRYPOINT ["/app/mcpi-server"]