# AGENTS.md

## Stack

| Layer       | Tech                                              |
|-------------|---------------------------------------------------|
| Backend     | Pure Rust (stable toolchain)                      |
| Frontend    | Rust → WASM via `wasm-pack` / `wasm-bindgen`      |
| Glue        | Minimal JS only — no JS for business logic        |
| Async       | `tokio` (backend), `wasm-bindgen-futures` (WASM)  |
| Errors      | `thiserror` (libraries) · `anyhow` (binaries)     |
| Persistence | PostgreSQL 16 — runs in Docker only               |
| Runtime     | Docker — **nothing runs locally**, ever           |
| Orchestration | Docker Compose (dev + prod profiles)            |

---

## Commands

### Docker Compose — primary workflow (use these, not raw cargo)

```bash
# --- Bring everything up ---
docker compose up                                  # all services (db + backend + frontend)
docker compose up --build                          # force image rebuild
docker compose up backend                          # single service only

# --- TDD watch loop inside Docker ---
docker compose run --rm dev cargo watch -x "nextest run"
docker compose run --rm dev cargo watch -x "nextest run -- my::module"

# --- Run tests inside Docker ---
docker compose run --rm dev cargo nextest run
docker compose run --rm dev cargo nextest run --test integration
docker compose run --rm dev cargo nextest run -p backend
docker compose run --rm dev wasm-pack test --headless --chrome -- --test wasm

# --- Lint / format inside Docker ---
docker compose run --rm dev cargo clippy -- -D warnings
docker compose run --rm dev cargo fmt --check
docker compose run --rm dev cargo fmt

# --- Quality gates inside Docker ---
docker compose run --rm dev cargo test --doc
docker compose run --rm dev cargo audit
docker compose run --rm dev cargo llvm-cov --lcov --output-path lcov.info

# --- Database ---
docker compose up -d db                            # start Postgres only
docker compose exec db psql -U app -d appdb        # open psql shell

# --- Teardown ---
docker compose down                                # stop containers, keep volumes
docker compose down -v                             # stop + wipe volumes (full reset)
```

> **Rule**: never run `cargo` commands directly on the host. Always go through `docker compose run --rm dev`.
> The `dev` service mounts the workspace as a volume so edits are reflected instantly without rebuilds.

> **TDD cycle**: edit file on host → `cargo watch` inside `dev` container catches it → red → green → refactor.

---

## Docker Architecture

### Multi-stage build strategy — ship binaries only

Every image must follow this pattern: **build stage → runtime stage**.
The runtime image contains only the compiled binary (or WASM + static files). No Rust toolchain, no source code, no `cargo`.

#### Backend Dockerfile
```dockerfile
# ── Stage 1: builder ──────────────────────────────────────────────────────────
FROM rust:slim AS builder
WORKDIR /app

# Cache dependencies separately from source (layer efficiency)
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/
COPY core/Cargo.toml core/
RUN mkdir -p backend/src core/src \
    && echo "fn main(){}" > backend/src/main.rs \
    && echo "" > core/src/lib.rs \
    && cargo build --release -p backend \
    && rm -rf backend/src core/src

# Build real source
COPY . .
RUN touch backend/src/main.rs core/src/lib.rs \
    && cargo build --release -p backend

# ── Stage 2: runtime ──────────────────────────────────────────────────────────
FROM gcr.io/distroless/cc-debian12 AS runtime
COPY --from=builder /app/target/release/backend /usr/local/bin/backend
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/backend"]
```

#### Frontend Dockerfile
```dockerfile
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
```

### Layer caching rules (agents must follow)
- **Always copy `Cargo.toml` / `Cargo.lock` before source files.** This ensures `cargo build` dependencies are cached and only invalidated when dependencies change.
- **Touch the entry point after copying source** to force Cargo to rebuild only the affected crate.
- Never `COPY . .` as the first step.
- Do not install dev tools (`cargo-watch`, `wasm-pack` dev deps) in production images.

---

## Docker Compose Structure

```yaml
# docker-compose.yml — canonical structure

services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: app
      POSTGRES_PASSWORD: ${DB_PASSWORD}   # always from .env
      POSTGRES_DB: appdb
    volumes:
      - db_data:/var/lib/postgresql/data
      - ./migrations:/docker-entrypoint-initdb.d   # auto-run on first start
    ports:
      - "5432:5432"                                # dev only; remove in prod profile
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U app"]
      interval: 5s
      retries: 5

  backend:
    build:
      context: .
      dockerfile: backend/Dockerfile
      target: runtime                              # always target the slim stage
    environment:
      DATABASE_URL: postgres://app:${DB_PASSWORD}@db:5432/appdb
    ports:
      - "8080:8080"
    depends_on:
      db:
        condition: service_healthy

  frontend:
    build:
      context: .
      dockerfile: frontend/Dockerfile
      target: runtime
    ports:
      - "80:80"
    depends_on:
      - backend

  dev:                                             # development-only service
    build:
      context: .
      dockerfile: dev.Dockerfile                   # Rust + wasm-pack + cargo-watch
    volumes:
      - .:/app                                     # mount workspace for live editing
      - cargo_cache:/usr/local/cargo/registry      # cache registry between runs
      - target_cache:/app/target                   # cache build artifacts
    environment:
      DATABASE_URL: postgres://app:${DB_PASSWORD}@db:5432/appdb
    depends_on:
      db:
        condition: service_healthy
    profiles: [dev]                                # not started by default

volumes:
  db_data:
  cargo_cache:
  target_cache:
```

### Environment and secrets
- All secrets go in `.env` (never committed). A `.env.example` with dummy values is committed.
- Never hardcode `DATABASE_URL`, passwords, or keys in Dockerfiles or source code.
- Services read config via environment variables only — no config files baked into images.

```
# .env.example (committed)
DB_PASSWORD=change_me
APP_SECRET=change_me
```

### Profiles
| Profile   | Services started              | Use for          |
|-----------|-------------------------------|------------------|
| (default) | `db`, `backend`, `frontend`   | Integration / QA |
| `dev`     | `db`, `dev`                   | TDD inner loop   |

---

## Testing Framework Decisions

### Test runner — `cargo nextest`
Drop-in replacement for `cargo test`. Faster, cleaner output, better failure isolation.
Never use `cargo test` directly; always use `cargo nextest run`.

### Testing layers — use all four

| Layer            | Crate / mechanism              | When to use                                    |
|------------------|-------------------------------|------------------------------------------------|
| Unit             | `#[test]` in `mod tests {}`   | Every pure function, every module              |
| Parametrized     | `rstest` `#[rstest]`          | Same logic, multiple input sets                |
| Property-based   | `proptest`                    | Algorithmic correctness, parser inputs, math   |
| Snapshot         | `insta`                       | Complex structs, serialized output, API shapes |
| Integration      | `tests/` directory            | Cross-module and API boundary tests            |
| WASM             | `wasm-bindgen-test`           | Any `#[wasm_bindgen]` surface                  |
| Benchmarks       | `criterion`                   | Hot paths — run separately, never in CI        |
| Mocking          | `mockall`                     | Trait-based dependency injection only          |

### Test-first rule (TDD)
1. **Red** — write the test, confirm it fails to compile or asserts falsely.
2. **Green** — write the minimum code to pass.
3. **Refactor** — clean up; tests must stay green.

Do not write implementation code without a failing test first.

### Placement rules
```
src/
  lib.rs
  foo.rs          ← unit tests live inside the module they test
    #[cfg(test)]
    mod tests { ... }

tests/
  integration/    ← cross-crate integration tests

benches/          ← criterion benchmarks (never in CI)
```

### `proptest` — required for
- Parsers and deserializers
- Any function claiming a mathematical property (commutativity, idempotency, etc.)
- Validation logic

### `insta` snapshots — required for
- API response shapes
- Complex serialized structs
- Error message formatting

Always commit `.snap` files. Review snapshot diffs carefully — never blindly `cargo insta accept`.

---

## Code Style & Rust Best Practices

### Error handling
```rust
// ✅ Libraries: typed, structured errors
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("not found: {0}")]
    NotFound(String),
}

// ✅ Binaries / main: anyhow for ergonomic propagation
fn main() -> anyhow::Result<()> { ... }

// ❌ Never
fn do_thing() -> Result<(), Box<dyn std::error::Error>> { ... }
fn do_thing() -> Option<T> { ... } // when error context matters
```

### No panics in library code
```rust
// ❌ Never in lib code
unwrap(), expect("..."), panic!()

// ✅ Use
.ok_or(AppError::NotFound(...))?
.map_err(|e| AppError::Io(e))?
```
The only acceptable `unwrap()` is inside `#[cfg(test)]` blocks.

### Ownership and borrowing
- Prefer `&str` over `&String`, `&[T]` over `&Vec<T>` in function signatures.
- Clone only when necessary; document why with a comment if non-obvious.
- Use `Arc<T>` / `Mutex<T>` for shared state; avoid global mutable state.

### Traits and generics
- Define behaviour through traits, not concrete types.
- Use `mockall::automock` on traits that need to be injected, not on concrete structs.
- Keep trait bounds at the `impl` site, not on struct definitions unless required.

### Naming
| What             | Convention          |
|------------------|---------------------|
| Types / Traits   | `PascalCase`        |
| Functions / vars | `snake_case`        |
| Constants        | `SCREAMING_SNAKE`   |
| Modules          | `snake_case`        |
| WASM exports     | `camelCase` (JS API) |

### WASM-specific rules
- No `std::fs`, `std::net`, or threading primitives in WASM crates — they don't compile.
- All WASM-exported types must be `JsValue`-compatible or derive `Serialize/Deserialize`.
- Use `#[cfg(target_arch = "wasm32")]` guards, not separate files.
- JS glue file must import from the WASM package only — no inline business logic.

```rust
// ✅ Correct WASM export
#[wasm_bindgen]
pub fn compute(input: &str) -> Result<String, JsValue> {
    core::do_work(input)
        .map_err(|e| JsValue::from_str(&e.to_string()))
}
```

### Clippy — zero warnings
Code must compile with `cargo clippy -- -D warnings`. Suppressions require a comment:
```rust
#[allow(clippy::too_many_arguments)] // justified: mirrors external protocol struct
```

---

## Project Structure (Workspace)

```
/
├── AGENTS.md
├── Cargo.toml              ← workspace root
├── docker-compose.yml      ← canonical; all services defined here
├── .env.example            ← committed; .env is gitignored
├── dev.Dockerfile          ← dev image: Rust + wasm-pack + cargo-watch
├── backend/
│   ├── Dockerfile          ← multi-stage: builder → distroless runtime
│   ├── src/
│   └── tests/
├── frontend/
│   ├── Dockerfile          ← multi-stage: wasm-builder → nginx:alpine
│   ├── src/
│   └── tests/
├── core/                   ← shared pure Rust logic (no I/O, WASM-safe)
│   ├── src/
│   └── tests/
├── js-glue/                ← minimal JS; imports from wasm-pack output only
└── migrations/             ← SQL migration files (auto-run by Postgres on first start)
    ├── 001_init.sql
    └── 002_...sql
```

> `core/` must compile for both native and `wasm32-unknown-unknown` targets.
> Run inside dev container: `cargo check --target wasm32-unknown-unknown -p core`

---

## Git Workflow

### Commit format (Conventional Commits)
```
feat(backend): add user authentication endpoint
fix(wasm): handle null input in compute export
test(core): add proptest for parser round-trip
refactor(frontend): extract validation into core
chore: update wasm-pack to 0.13
```

### Branch naming
```
feat/user-auth
fix/wasm-null-crash
test/parser-proptest
```

### Rules
- Never commit directly to `main`.
- Every PR must have a passing `cargo nextest run` and `cargo clippy -- -D warnings`.
- Do not merge if `cargo audit` reports a vulnerability in direct dependencies.
- Snapshot file changes (`.snap`) must be reviewed line-by-line in PR.

---

## Boundaries

| ✅ Do freely                                      | ⚠️ Ask first                            | 🚫 Never                                      |
|--------------------------------------------------|-----------------------------------------|-----------------------------------------------|
| Read any file                                    | Add a new workspace crate               | Run `cargo` directly on the host              |
| `docker compose run --rm dev cargo nextest run`  | Add a new external dependency           | Commit `.env` or any secret                   |
| `docker compose up --build`                      | Modify `Cargo.toml` dependency versions | Hardcode secrets in Dockerfiles or source     |
| Write or update tests                            | Change error type signatures            | `unwrap()` / `expect()` in library code       |
| Update snapshot files after review               | Modify public API surface               | Business logic in JS glue files               |
| `cargo audit` / `cargo llvm-cov` (in container) | `git push` / open PRs                   | Skip or `#[ignore]` a test without reason     |
| `docker compose down`                            | `docker compose down -v` (wipes data)   | `COPY . .` as first step in a Dockerfile      |
| Edit migrations (new file only)                  | Modify an existing migration file       | Use `unsafe` without documented invariants    |

---

## Key Dependencies (Dev)

```toml
[dev-dependencies]
rstest      = "0.23"    # parametrized tests
proptest    = "1.9"     # property-based tests
insta       = "1"       # snapshot tests
mockall     = "0.13"    # trait mocking
criterion   = "0.5"     # benchmarks (benches/ only)

[dev-dependencies] # wasm crate only
wasm-bindgen-test = "0.3"
```