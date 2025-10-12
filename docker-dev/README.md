# Docker Development Environment

This directory contains Docker configuration for local development of the eddist project.

## Setup

### 1. Create `.env` file

Copy the example file and configure for your platform:

```bash
cp .env.example .env
```

### 2. Configure `HOST_GATEWAY_IP` for your platform

Edit `.env` and set the appropriate value:

#### macOS (Docker Desktop)

```env
HOST_GATEWAY_IP=host.docker.internal
```

Docker Desktop on Mac has built-in support for `host.docker.internal`.

#### WSL2 (Windows)

Find your Docker gateway IP:

```bash
docker network inspect bridge | grep Gateway
```

Then set it in `.env`:

```env
HOST_GATEWAY_IP=172.18.0.1
```

(The IP may be `172.17.0.1` or `172.18.0.1` depending on your network)

#### Linux (native Docker)

Find your Docker bridge gateway:

```bash
docker network inspect bridge | grep Gateway
```

Then set it in `.env`:

```env
HOST_GATEWAY_IP=172.17.0.1
```

### 3. Start the services

```bash
docker compose up
```

For background mode:

```bash
docker compose up -d
```

## Accessing the Application

- **Main application**: http://localhost:8000 (via nginx)
- **Frontend (direct)**: http://localhost:5173 (Vite dev server)
- **phpMyAdmin**: http://localhost:8082
- **Backend API**: http://localhost:8080 (runs on host, not in Docker)

## Development Workflow

The client-v2 service mounts your workspace and supports hot reloading:

1. Edit files in `eddist-server/client-v2/`
2. Changes are automatically reflected in the running container
3. Vite will hot-reload the browser

## Stopping Services

```bash
docker compose down
```

To also remove volumes:

```bash
docker compose down -v
```

## Troubleshooting

### Cannot connect to backend from container

Verify `HOST_GATEWAY_IP` is correct:

```bash
docker exec client_v2_container getent hosts host.docker.internal
```

Test connectivity:

```bash
docker exec client_v2_container node -e "fetch('http://host.docker.internal:8080/api/boards').then(r => console.log('OK:', r.status)).catch(e => console.log('Error:', e.message))"
```

### pnpm install fails

The `CI=true` environment variable prevents interactive prompts. If you need to reset:

```bash
docker compose down -v
docker compose up
```
