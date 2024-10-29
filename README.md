# Eddist
Anonymous BBS System running on Container

## Features
- Simple threadfloat BBS
- TODO: rich management page

## Architecture

```mermaid
flowchart
  U[User]
  R[Redis]
  DB[MySQL]
  E[eddist]
  P[eddist-persistence]

  U --> E
  E --> R
  R --> E
  E --> DB
  R --> P
  P --> R
  P --> DB
```

## Usage
- Docker compose
  1. (Optional) Create Auth0 account and get some credentials for `.docker-compose.env`
  2. Fill placeholders in `.docker-compose.env`
  3. Install docker and docker-compose, Rust
  4. Execute `cargo install sqlx-cli` for database migration
  5. Create `captcha-config.json` with following content:
      ```json
      []
      ```
  6. Execute `docker-compose up -d`
  7. Run `DATABASE_URL=mysql://root:rootpassword@localhost:3306/eddist sqlx database create` and `DATABASE_URL=mysql://root:rootpassword@localhost:3306/eddist sqlx migrate run` to initialize database
- Other options
  - You can run `eddist` binary directly, or run on other container orchestration tools (e.g., Kubernetes) and PaaS.
    - We don't provide any support for this.  

## License
AGPL v3
