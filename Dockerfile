# Image de déploiement de Weave : l'API et le site vitrine dans un seul
# processus, donc un seul dyno à payer.
#
# L'image de base est l'image officielle Node.js, en version figée : la même
# que celle du développement et de l'intégration continue.

FROM node:22.20-slim AS base
WORKDIR /app

# ------------------------------------------------------------------
# Dépendances — les manifestes seuls, pour que la couche reste en cache
# tant qu'ils ne changent pas.
# ------------------------------------------------------------------
FROM base AS deps
COPY package.json package-lock.json ./
COPY apps/api/package.json apps/api/package.json
COPY apps/web/package.json apps/web/package.json
COPY packages/contracts/package.json packages/contracts/package.json
RUN npm ci

# ------------------------------------------------------------------
# Construction — clients Prisma et site vitrine.
# ------------------------------------------------------------------
FROM deps AS build
COPY . .
RUN npm run db:sqlite \
 && npm run db:generate \
 && npm run build --workspace @weave/web \
 # Les dépendances de développement ne servent plus : l'API est exécutée
 # depuis ses sources TypeScript, que Node lit nativement.
 && npm prune --omit=dev

# ------------------------------------------------------------------
# Exécution
# ------------------------------------------------------------------
FROM base AS runtime

ENV NODE_ENV=production \
    WEAVE_DB=postgres \
    WEB_DIST_PATH=/app/apps/web/dist

COPY --from=build /app /app

# L'image officielle Node fournit déjà un utilisateur non privilégié.
USER node

EXPOSE 3000
CMD ["node", "apps/api/src/index.ts"]
