# Use official Rust image for consistent build environment
FROM rust:1.75-bullseye as builder

# Install dependencies
RUN apt-get update && \
    apt-get install -y \
    cmake \
    pkg-config \
    libssl-dev \
    git \
    clang \
    libclang-dev \
    protobuf-compiler \
    libprotobuf-dev \
    libsecp256k1-dev

WORKDIR /throughput

# Copy source code
COPY . .

# Remove any local cargo config that might have macOS-specific settings
RUN rm -rf .cargo/config.toml .cargo/config

# Set proper compiler for Linux container
ENV CC=clang
ENV CXX=clang++
ENV AR=ar
ENV RUSTFLAGS=""
ENV LDFLAGS=""
ENV CPPFLAGS=""

# Build the node
RUN cargo build --release --bin frontier-template-node

# Runtime image
FROM debian:bullseye-slim

RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    libssl1.1 \
    libsecp256k1-0 && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /throughput/target/release/frontier-template-node /usr/local/bin/

EXPOSE 9944 9933 30333

CMD ["frontier-template-node", "--dev"]