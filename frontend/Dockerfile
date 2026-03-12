# ── Stage 1: WASM builder ─────────────────────────────────────────────────────
FROM rust:slim AS wasm-builder
RUN cargo install wasm-pack
WORKDIR /app
COPY . .
RUN wasm-pack build --target web --release frontend/

# ── Stage 2: static file server ───────────────────────────────────────────────
FROM nginx:alpine AS runtime
COPY --from=wasm-builder /app/js-glue/ /usr/share/nginx/html/
COPY --from=wasm-builder /app/frontend/pkg/ /usr/share/nginx/html/pkg/
EXPOSE 80
