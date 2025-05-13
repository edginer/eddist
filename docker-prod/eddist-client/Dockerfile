FROM node:22-bookworm-slim

# Setup pnpm using Corepack
ENV PMPM_HOME="/pnpm"
ENV PATH="${PMPM_HOME}:${PATH}"
RUN corepack enable

COPY pnpm-lock.yaml pnpm-workspace.yaml package.json /app/
COPY eddist-server/client-v2/package.json /app/eddist-server/client-v2/
COPY node_modules /app/
COPY eddist-server/client-v2/node_modules /app/eddist-server/client-v2/
COPY eddist-server/client-v2/app /app/eddist-server/client-v2/app
COPY eddist-server/client-v2/build /app/eddist-server/client-v2/build

WORKDIR /app/

RUN ["pnpm", "install", "--prod"]

CMD ["pnpm", "-F", "eddist-client-v2", "start"]
