FROM rust:slim

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    git \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

RUN cargo install cargo-watch wasm-pack cargo-nextest

WORKDIR /app
