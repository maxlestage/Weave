# Image de déploiement de Weave : l'API et le site vitrine dans un seul
# processus, donc un seul dyno à payer.
#
# L'image de base est l'image officielle Bun, en version figée : la même que
# celle du développement et de l'intégration continue.

FROM oven/bun:1.4.2-slim AS base
WORKDIR /app

# ------------------------------------------------------------------
# Dépendances — copiées seules, pour que la couche reste en cache tant
# que les manifestes ne changent pas.
# ------------------------------------------------------------------
FROM base AS deps
COPY package.json bun.lock bunfig.toml ./
COPY apps/api/package.json apps/api/package.json
COPY apps/web/package.json apps/web/package.json
COPY packages/contracts/package.json packages/contracts/package.json
COPY packages/prisma-bun-sqlite/package.json packages/prisma-bun-sqlite/package.json
RUN bun install --frozen-lockfile

# ------------------------------------------------------------------
# Construction — clients Prisma et site vitrine.
# ------------------------------------------------------------------
FROM deps AS build
COPY . .
RUN bun run db:sqlite \
 && bun run db:generate \
 && bun run --filter @weave/web build

# ------------------------------------------------------------------
# Exécution
# ------------------------------------------------------------------
FROM base AS runtime

ENV NODE_ENV=production \
    WEAVE_DB=postgres \
    WEB_DIST_PATH=/app/apps/web/dist

COPY --from=build /app /app

# L'image officielle Bun fournit déjà un utilisateur non privilégié.
USER bun

EXPOSE 3000
CMD ["bun", "apps/api/src/index.ts"]
