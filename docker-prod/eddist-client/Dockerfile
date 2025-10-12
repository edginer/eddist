FROM --platform=amd64 node:22-bookworm-slim AS dependencies

# Setup pnpm using Corepack
ENV PNPM_HOME="/pnpm"
ENV PATH="${PNPM_HOME}:${PATH}"
RUN corepack enable

# Copy package files for dependency installation
COPY pnpm-lock.yaml pnpm-workspace.yaml package.json /app/
COPY eddist-server/client-v2/package.json /app/eddist-server/client-v2/

WORKDIR /app

# Install production dependencies only
RUN pnpm install --prod --frozen-lockfile

FROM --platform=amd64 node:22-bookworm-slim

# Setup pnpm using Corepack
ENV PNPM_HOME="/pnpm"
ENV PATH="${PNPM_HOME}:${PATH}"
RUN corepack enable

WORKDIR /app

# Copy workspace configuration
COPY pnpm-workspace.yaml package.json /app/

# Copy production dependencies from build stage
COPY --from=dependencies /app/node_modules /app/node_modules
COPY --from=dependencies /app/eddist-server/client-v2/node_modules /app/eddist-server/client-v2/node_modules

# Copy application files (pre-built from CI)
COPY eddist-server/client-v2/package.json /app/eddist-server/client-v2/
COPY eddist-server/client-v2/server.js /app/eddist-server/client-v2/
COPY eddist-server/client-v2/build /app/eddist-server/client-v2/build

CMD ["pnpm", "-F", "eddist-client-v2", "start"]
