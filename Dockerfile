# Image de déploiement de Weave : l'API et le site vitrine dans un seul
# processus, donc un seul dyno à payer.
#
# L'API est un binaire Rust ; le site est bâti par Bun. Deux chaînes de
# construction, une seule image d'exécution — et elle ne porte ni l'une ni
# l'autre, seulement ce qu'elles produisent.

# ------------------------------------------------------------------
# Le site vitrine
# ------------------------------------------------------------------
FROM oven/bun:1.4.2-slim AS site
WORKDIR /app
COPY package.json bun.lock bunfig.toml ./
COPY apps/web/package.json apps/web/package.json
COPY packages/contracts/package.json packages/contracts/package.json
RUN bun install --frozen-lockfile
COPY . .
RUN bun run --filter @weave/web build

# ------------------------------------------------------------------
# L'API
#
# Les manifestes sont copiés seuls d'abord : la couche des dépendances reste
# en cache tant qu'ils ne changent pas, et compiler les dépendances de Weave
# coûte plusieurs minutes.
# ------------------------------------------------------------------
# La version de Debian est épinglée des deux côtés, et c'est le point
# important : `rust:1-slim` suit la version par défaut de l'image Rust, qui est
# passée à Debian 13. Le binaire réclamait alors `GLIBC_2.38`, absent de
# l'étage d'exécution sous Debian 12 — l'image se construisait sans erreur, et
# le dyno s'arrêtait au démarrage sur un message que rien ne relie à la
# construction. Les deux étages doivent nommer la même version.
FROM rust:1-slim-bookworm AS api
WORKDIR /build
COPY apps/api-rs/Cargo.toml apps/api-rs/Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release && rm -rf src
COPY apps/api-rs/ ./
RUN touch src/main.rs && cargo build --release

# ------------------------------------------------------------------
# Exécution
# ------------------------------------------------------------------
FROM debian:bookworm-slim AS runtime
WORKDIR /app

# Le binaire n'a besoin que de la libc et des certificats racine : TLS passe
# par rustls, pas par OpenSSL — `ldd` sur le binaire construit ne montre ni
# libssl ni libcrypto. Installer OpenSSL ici alourdirait l'image sans que rien
# ne l'ouvre jamais.
RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

ENV NODE_ENV=production \
    WEAVE_DB=postgres \
    WEB_DIST_PATH=/app/apps/web/dist

COPY --from=api /build/target/release/weave-api /app/bin-release/weave-api
COPY --from=site /app/apps/web/dist /app/apps/web/dist

RUN useradd --system --uid 10001 weave && chown -R weave /app
USER weave

EXPOSE 3000
CMD ["/app/bin-release/weave-api"]
