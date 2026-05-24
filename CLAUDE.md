# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Eddist is an anonymous BBS (bulletin board system) running on containers, similar to 2channel/5channel. It's a monorepo containing multiple Rust services and React frontends using a pnpm workspace.

## Workspace Structure

This is a **Cargo workspace** with 5 Rust crates:

- **eddist-server** - Main BBS application server (port 8080)
  - Handles thread/response posting via `/test/bbs.cgi` (Shift-JIS encoding)
  - Serves client-v2 React SSR application
- **eddist-admin** - Admin panel backend (port 8081) + React frontend
  - REST API for managing boards, users, terms, notices, auth tokens, etc.
  - React Router 7 frontend with Flowbite UI
  - OpenAPI schema at `/openapi.json`
- **eddist-core** - Shared utilities and domain models
- **eddist-persistence** - Background job for Redis→MySQL persistence
- **eddist-cron** - Scheduled tasks

Frontend applications use **pnpm workspaces** (not npm).

## Build & Run Commands

### Database Setup

```bash
# Install sqlx-cli first
cargo install sqlx-cli --no-default-features --features mysql

# Create database and run migrations
DATABASE_URL=mysql://root:rootpassword@localhost:3306/eddist sqlx database create
DATABASE_URL=mysql://root:rootpassword@localhost:3306/eddist sqlx migrate run
```

### Rust Services

```bash
# Build all workspace members
cargo build

# Run specific service
cargo run -p eddist
cargo run -p eddist-admin

# Check compilation (faster than build)
cargo check
```

### Frontend Development

```bash
# Install dependencies (use pnpm, NOT npm)
pnpm install

# Run admin client dev server
pnpm -F eddist-admin-client dev

# Build admin client
pnpm -F eddist-admin-client build

# Run client-v2 (from eddist-server/client-v2)
pnpm -F eddist-client-v2 dev
pnpm -F eddist-client-v2 build
```

### Docker Development Environment

```bash
# Start all services (MySQL, Redis, phpMyAdmin, nginx, client-v2)
docker compose -f docker-dev/docker-compose.yml up

# Configure HOST_GATEWAY_IP in docker-dev/.env first:
# - macOS: host.docker.internal
# - WSL2: 172.18.0.1 (check with: docker network inspect bridge | grep Gateway)
# - Linux: 172.17.0.1
```

### Testing

```bash
# Run Rust tests
cargo test

# Run specific crate tests
cargo test -p eddist
cargo test -p eddist-core

# Run integration tests (MUST use single thread due to testcontainers)
cargo test --test integration_test -- --test-threads=1

# Run with logging
RUST_LOG=debug cargo test
```

## Architecture & Data Flow

### Request Flow

```
User → nginx (port 8000)
  ├→ /api/* → eddist-server (port 8080)
  ├→ /test/bbs.cgi → eddist-server (Shift-JIS BBS endpoint)
  └→ / → client-v2 SSR (React Router 7)

Admin → eddist-admin (port 8081)
  ├→ /api/* → Rust backend (Axum)
  └→ /dashboard/* → React frontend (React Router 7)
```

### Database & Cache

- **MySQL**: Primary data store (threads, responses, users, boards)
- **Redis**: Cache layer + ephemeral data
  - Thread caches: `thread_cache:{board_key}:{thread_number}`
  - Suspended tokens: `authed_token:suspended:{id}` (TTL-based temporary suspension)
  - Rate limiting, sessions, CSRF state

### Internal API (eddist-admin)

`/internal/api` routes are separate from the standard `/api` routes and require an `X-Internal-Secret` header matching the `EDDIST_INTERNAL_SECRET` env var. If `EDDIST_INTERNAL_SECRET` is not set, internal routes return `503 Service Unavailable`.

Current internal routes:
- `POST /internal/api/authed-tokens/suspend` — temporarily suspends a token via Redis TTL key `authed_token:suspended:{id}`

## Key Technical Patterns

### Linting

```bash
cargo clippy
cargo clippy -p eddist
```

### Shift-JIS Encoding

BBS endpoints use Shift-JIS (Japanese legacy encoding) for compatibility with 2channel clients:

```rust
// In routes/bbs_cgi.rs
let form = shift_jis_url_encodeded_body_to_vec(&body)?;
let response = SJisResponseBuilder::new(SJisStr::from(html))
    .content_type(SjisContentType::TextHtml)
    .build();
```

### Service Layer Pattern

Services implement `AppService<Input, Output>` or `BbsCgiService<Input, Output>` traits:

```rust
// Example: thread_creation_service.rs
impl BbsCgiService<TheradCreationServiceInput, ThreadCreationServiceOutput> {
    async fn execute(&self, input: Input) -> Result<Output, BbsCgiError>
}
```

Services are grouped in `AppServiceContainer` and accessed via `State(state).services.thread_creation()`.

### Repository Pattern

All database access goes through repository traits (e.g., `BbsRepository`, `AuthedTokenRepository`). Implementations use sqlx with compile-time query verification.

### React Router File-Based Routing

Admin UI uses React Router 7 file-based routing:
- `dashboard.boards.tsx` → `/dashboard/boards`
- `dashboard.boards_.$boardKey.tsx` → `/dashboard/boards/:boardKey`
- Nested routes use underscore to avoid parent layout nesting: `dashboard.boards_.$boardKey.tsx`

### client-v2 State Management

client-v2 uses a dual persistence approach:
- **IndexedDB** (`utils/idb.ts`): Structured data — `read_history` (max 500, auto-pruned), `favorites`, `post_history`. Browser-only; guarded with `typeof window !== "undefined"`.
- **localStorage**: Simple KV settings — UI flags (`eddist:ui:settings`) and theme (`eddist:theme`).

Five React contexts wrap these stores (all in `app/contexts/`):
- `ThreadHistoryContext` — syncs IndexedDB read/favorite/post history into React state; cross-store sync (favorites ↔ history)
- `UISettingsContext` — persists feature flags (showHistoryButtons, enableReadHistory, enableFavorites, enablePostHistory)
- `ThemeContext` — three-state theme (light/dark/system) with `matchMedia` listener for system preference changes
- `NGWordsContext` — NG word filter with regex caching, debounced saves (300ms), and cross-tab sync via `StorageEvent`
- `ToastProvider` — auto-dismissing notifications (3s timeout)

### client-v2 SSR-Aware API Clients

API clients in `app/api-client/` detect `import.meta.env.SSR`:
- **Server-side**: absolute base URL, in-memory cache with 60s TTL
- **Client-side**: relative paths, no caching layer

### eddist-cron Job Types

`eddist-cron` is a CLI job runner with four commands, executed per board with random jitter (0–59s) to avoid thundering herd:
- `inactivate` — archives threads when cron schedule matches and thread count exceeds threshold
- `archive` — moves threads from main table to archive table
- `convert` — converts threads to DAT format (Shift-JIS bytes) and uploads to Cloudflare R2; deletes Redis cache key `thread:{board}:{number}` on success
- `backfill-convert` — re-uploads threads missing from R2 (detected via S3 HEAD requests), with exponential backoff (2s→4s→8s→16s)

### OpenAPI Integration (eddist-admin only)

The admin service generates OpenAPI specs using `utoipa`:

```rust
#[utoipa::path(
    get,
    path = "/plugins",
    tag = "plugins",
    responses(...)
)]
async fn list_plugins() { ... }
```

Client uses `openapi-fetch` with generated types from `openapi.json`.

#### Regenerating OpenAPI Schema and TypeScript Types

After modifying API routes or models in eddist-admin:

```bash
# 1. Generate openapi.json from Rust code (run from workspace root)
cargo run -p eddist-admin -- --openapi

# 2. Generate TypeScript types from openapi.json
pnpm dlx openapi-typescript ./eddist-admin/openapi.json -o ./eddist-admin/client/app/openapi/schema.d.ts
```

**Important:** Never manually edit `openapi.json` - always regenerate from Rust code.

## Important Implementation Details

### Migration Workflow

Always use sqlx migrations:

```bash
# Create new migration
sqlx migrate add migration_name

# Apply migrations
DATABASE_URL=mysql://... sqlx migrate run

# Revert last migration
DATABASE_URL=mysql://... sqlx migrate revert
```

### AppState Management

Both servers use generic AppState patterns:

**eddist-server**: Struct with `AppServiceContainer` (concrete generic params), `notice_repo`, `terms_repo`, `template_engine`, `tinker_secret`
**eddist-admin**: Struct with `Arc<dyn Trait>` repository fields for dynamic dispatch + `redis_conn` + `internal_secret`

Access state in route handlers: `State(state): State<AppState>`

### Integration Test Infrastructure

Integration tests (in `eddist-server/tests/`) use `testcontainers` to spin up real MySQL 8.0 and Redis/Valkey 8.0 containers. `TestContext` wraps the DB pool, Redis connection, and an `axum-test` `TestServer`. Migrations run automatically via `sqlx::migrate!`. Tests submit Shift-JIS encoded forms and simulate `CF-Connecting-IP` / `X-ASN-Num` headers. **Must run with `--test-threads=1`** due to shared container state.

## Environment Variables

Key environment variables (see `.env` or `.docker-compose.env`):

- `DATABASE_URL` - MySQL connection string
- `REDIS_URL` - Redis connection string
- `RUST_LOG` - Log level (debug, info, warn, error)
- `RUST_ENV` - Environment (prod/production for production)
- `BBS_NAME` - Display name for the BBS
- `ENABLE_USER_REGISTRATION` - Enable user registration feature
- `TINKER_SECRET` - Secret for tinker endpoint (eddist-server)
- `S3_BUCKET_NAME`, `R2_ACCOUNT_ID`, `S3_ACCESS_KEY`, `S3_ACCESS_SECRET_KEY` - Cloudflare R2 storage (both services)
- `EDDIST_ADMIN_CLIENT_ID`, `EDDIST_ADMIN_CLIENT_SECRET`, `EDDIST_ADMIN_AUTH_URL`, `EDDIST_ADMIN_TOKEN_URL`, `EDDIST_ADMIN_LOGIN_CALLBACK_URL` - OAuth2/OIDC for admin auth
- `EDDIST_INTERNAL_SECRET` - Shared secret for `/internal/api` routes (eddist-admin); routes return 503 if unset
- `AXUM_METRICS` - Set to `true` to enable Prometheus metrics middleware on `/metrics` (eddist-server)
- `DISABLE_METRICS` - Set to `true` to disable metrics entirely (eddist-server)

## Common Gotchas

1. **Always use pnpm**, not npm or yarn for frontend
2. **Migrations must be run** from workspace root, not crate directories
3. **Token suspension** writes a TTL key to Redis (`authed_token:suspended:{id}`); the check is separate from the DB `validity` field
4. **Shift-JIS encoding** is required for `/test/bbs.cgi` compatibility
5. **React Router 7** uses `react-router` package (not the older `react-router-dom`)
6. **File-based routing** in admin client uses underscores for path segments (e.g., `dashboard.boards_.$boardKey.tsx`)
7. **sqlx offline mode**: Run `cargo sqlx prepare` after schema changes for CI
8. **Thread IDs are timestamps**: Thread numbers are Unix timestamps in seconds, used for both identification and creation time
9. **Do not use `cd` commands**: Use `cargo run -p <crate>` or `pnpm -F <package>` instead of changing directories

## Documentation References

- Docker dev setup: `docker-dev/README.md`
- OpenAPI spec: `eddist-admin/openapi.json` (after running admin server)
- Database migrations: `migrations/` directory in workspace root
