# ── Stage 1: builder ──────────────────────────────────────────────────────────
FROM rust:slim AS builder
WORKDIR /app

# Cache dependencies separately from source (layer efficiency)
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/
COPY search_core/Cargo.toml search_core/
RUN mkdir -p backend/src search_core/src \
    && echo "fn main(){}" > backend/src/main.rs \
    && echo "" > search_core/src/lib.rs \
    && cargo build --release -p backend \
    && rm -rf backend/src search_core/src

# Build real source
COPY . .
RUN touch backend/src/main.rs search_core/src/lib.rs \
    && cargo build --release -p backend

# ── Stage 2: runtime ──────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12 AS runtime
COPY --from=builder /app/target/release/backend /usr/local/bin/backend
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/backend"]
