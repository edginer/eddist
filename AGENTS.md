# Repository guidance

This file provides guidance to coding agents working in this repository.

Eddist is an anonymous Japanese BBS (bulletin board system) running on containers. It's a Rust-based monorepo with React frontend components following a microservices architecture with Shift-JIS encoding support.

## Architecture

### Services (Rust Workspace)
- **eddist-server**: Main BBS server (Axum, port 8080)
- **eddist-admin**: Admin panel backend (Axum, port 8081)
- **eddist-persistence**: Background service persisting data from Redis to MySQL
- **eddist-cron**: Scheduled tasks (thread archiving to S3/R2)
- **eddist-core**: Shared domain logic, types, and utilities

### Frontend Clients (pnpm Workspace)
- **eddist-admin/client**: Admin panel UI (React Router v7, TypeScript, Tailwind, Flowbite)
- **eddist-server/client-v2**: SSR BBS client (React Router v7, Express, SWR)

### Data Flow
```
User → eddist-server → Redis ← eddist-persistence → MySQL
```

### Key Source Structure

**eddist-server/src/:**
- `routes/` - HTTP endpoint handlers (bbs_cgi, dat_routing, auth_code, etc.)
- `repositories/` - Data access layer (bbs_repository, bbs_cache_repository)
- `domain/` - Business logic services
- `middleware/` - Request middleware
- `resources/templates/` - Handlebars templates

**eddist-core/src/domain/:**
- Core entities: `board.rs`, `res.rs`, `thread.rs`, `cap.rs`
- Utilities: `sjis_str.rs`, `ip_addr.rs`, `tinker.rs`

**eddist-admin/src/:**
- `routes/` - API endpoints (boards, threads, moderation, users, etc.)
- `repository/` - Data access layer
- `models/` - Request/response DTOs
- OpenAPI spec generated via `utoipa`

## Development Commands

### Rust
```bash
cargo build                      # Build all services
cargo run -p eddist              # Run main server
cargo run -p eddist-admin        # Run admin backend
cargo test                       # Run all tests
cargo test -p eddist-core        # Test specific package
cargo test test_name              # Run single test by name
cargo clippy                     # Lint
cargo check                      # Fast type checking
```

### Frontend (pnpm monorepo)
```bash
pnpm install                              # Install all dependencies

# Both frontend clients use Biome for linting/formatting (not ESLint)

# Admin client
pnpm -F eddist-admin-client dev           # Dev server
pnpm -F eddist-admin-client build         # Production build
pnpm -F eddist-admin-client typecheck     # Type check
pnpm -F eddist-admin-client lint          # Biome lint
pnpm -F eddist-admin-client format        # Biome format
pnpm -F eddist-admin-client check         # Biome check (lint + format)

# BBS client v2
pnpm -F eddist-client-v2 dev              # Dev server
pnpm -F eddist-client-v2 build            # Production build
pnpm -F eddist-client-v2 start            # Run production SSR server
pnpm -F eddist-client-v2 typecheck        # Type check
pnpm -F eddist-client-v2 lint             # Biome lint
pnpm -F eddist-client-v2 format           # Biome format
pnpm -F eddist-client-v2 check            # Biome check (lint + format)
```

### Database
```bash
cargo install sqlx-cli                    # Install SQLX CLI (one-time)
sqlx database create                      # Create database
sqlx migrate run                          # Run migrations
```

### Docker Development
```bash
# Start infrastructure (MySQL, Redis, phpMyAdmin)
cd docker-dev && docker compose up -d

# Configure HOST_GATEWAY_IP in docker-dev/.env for your platform:
# macOS: host.docker.internal
# Linux/WSL2: 172.17.0.1 or 172.18.0.1

# Access points:
# - http://localhost:8000 (nginx proxy)
# - http://localhost:5173 (Vite dev server)
# - http://localhost:8082 (phpMyAdmin)
```

## Key Technical Details

### Backend
- **Framework**: Axum with Tokio async runtime
- **Database**: MySQL with SQLX (compile-time checked queries)
- **Cache/Pubsub**: Redis for caching and real-time updates
- **Auth**: Auth0/OIDC for admin, authed_tokens for users
- **Templating**: Handlebars for server-rendered pages
- **Encoding**: Shift-JIS support via `encoding_rs`

### Frontend
- **Routing**: React Router v7 with file-based routing
- **Data Fetching**: SWR (client-v2), TanStack Query (admin)
- **Styling**: Tailwind CSS v4, Flowbite components
- **Forms**: react-hook-form with zod validation
- **API Client**: openapi-fetch with generated types (admin)

### Configuration
- Environment variables via `.env` (copy from `.docker-compose.env`)
- CAPTCHA config in `captcha-config.json` (use `[]` to disable)
- Template config in `eddist-server/resources/templates.local.toml`

### Key Environment Variables
```bash
DATABASE_URL=mysql://root:rootpassword@localhost:3306/eddist
REDIS_URL=redis://localhost:6379
RUST_LOG=debug
BASE_URL=http://localhost:8080
USE_CLOUDFLARE_CDN=true    # Enables CF-Connecting-IP header
```

## Key Patterns

### Database Conventions
- All UUIDs stored as `BINARY(16)` — always use `BIN_TO_UUID()` / `UUID_TO_BIN()` in SQLX queries

### Repository Pattern
- `TransactionRepository<T: Database>` trait in `eddist-server/src/utils.rs` — use the `transaction_repository!` macro to implement it on repository structs
- `eddist-cron` is the exception: it uses raw structs instead of the repository trait

### Admin API Type Generation
The OpenAPI spec is generated from Rust (`utoipa`) at `eddist-admin/openapi.json`. To regenerate the TypeScript types after backend changes:
```bash
npx openapi-typescript eddist-admin/openapi.json -o eddist-admin/client/app/openapi/schema.d.ts
```

### Redis Key Naming
Canonical key constructors are in `eddist-server/src/utils.rs` under `pub(crate) mod redis`. Always use these functions rather than constructing keys inline.

## Agent Configuration

- This file is the shared source of repository guidance for Codex and Claude Code.
- `CLAUDE.md` imports this file with Claude Code's `@` file-reference syntax.
- Project skills are authored under `.agents/skills` and exposed to Claude Code through the `.claude/skills` symlink.
- Preserve unrelated user changes and run the smallest relevant verification command for each change.

## Important Notes
- Japanese BBS system - all user content uses Shift-JIS encoding
- Admin routes require Auth0 credentials for full functionality
- Migrations are in `/migrations/` directory
- AGPL v3 licensed
