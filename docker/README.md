# Docker images and Compose stacks

All Docker build contexts live under `docker/`. Build from the **repository root** so `Cargo.toml` and `crates/` are available.

## Dockerfiles

| File | Binary / image | Crate version tag |
|------|----------------|-------------------|
| `Dockerfile.broker` | `aai-broker` | `AAI_BROKER_VERSION` |
| `Dockerfile.visa-registry` | `visa-registry` | `VISA_REGISTRY_VERSION` |
| `Dockerfile.duo-service` | `duo-service` | `DUO_SERVICE_VERSION` |
| `Dockerfile.service-registry` | `service-registry` | `SERVICE_REGISTRY_VERSION` |
| `Dockerfile.all-in-one` | `ga4gh-infra` (combined CLI) | `GA4GH_INFRA_VERSION` |
| `Dockerfile.mock-idp` | `mock-idp` (dev/CI only) | `MOCK_IDP_VERSION` |
| `Dockerfile.sample-resource` | `sample-resource` | `SAMPLE_RESOURCE_VERSION` |
| `Dockerfile.register` | curl helper for one-shot registration | — |

Images use a multi-stage build (`rust:1-bookworm` → `debian:bookworm-slim`) and run as non-root user `ga4gh` (uid 1000). Runtime images include `curl` for Compose health checks.

Example manual build:

```bash
docker build -f docker/Dockerfile.broker -t ghcr.io/<org>/aai-broker:0.1.0 .
```

## Compose stacks

Copy version pins and set your registry prefix:

```bash
cp docker/.env.example docker/.env   # optional local overrides (gitignored)
```

Compose reads `--env-file docker/.env.example` by default in CI; use `docker/.env` locally if you customize pins.

| Compose file | Database | Use case |
|--------------|----------|----------|
| `docker-compose.yml` | PostgreSQL for visa-registry and service-registry | Full stack (CI, e2e, dev) |
| `docker-compose.sqlite.yml` | SQLite volume for visa-registry; PostgreSQL for service-registry only | Lighter local deployment |
| `docker-compose.prod.example.yml` | PostgreSQL; no mock-idp | Production reference (see [docs/production-deployment.md](../docs/production-deployment.md)) |

TLS termination examples: [`reverse-proxy/`](reverse-proxy/README.md).

Start the default stack:

```bash
just up
# or
docker compose -f docker/docker-compose.yml --env-file docker/.env.example up --build --wait
```

SQLite variant:

```bash
just up-sqlite
# or
docker compose -f docker/docker-compose.sqlite.yml --env-file docker/.env.example up --build --wait
```

### Version pins (`.env`)

Component image tags match crate versions independently (see [docs/versioning.md](../docs/versioning.md)):

```env
GA4GH_IMAGE_PREFIX=ghcr.io/synapticfour
AAI_BROKER_VERSION=0.1.0
VISA_REGISTRY_VERSION=0.1.0
# ...
```

Mix versions by editing `docker/.env` before `docker compose up`.

## CI releases

Pushing a git tag triggers `.github/workflows/docker-release.yml`:

| Git tag | Image pushed |
|---------|----------------|
| `aai-broker-v0.3.0` | `ghcr.io/<org>/aai-broker:0.3.0` (+ `:latest`) |
| `visa-registry-v0.1.5` | `ghcr.io/<org>/visa-registry:0.1.5` (+ `:latest`) |
| `ga4gh-infra-v0.4.0` | `ghcr.io/<org>/ga4gh-infra:0.4.0` (+ `:latest`) |

Replace `<org>` with your GitHub organization or username (lowercase).

## Layout

```text
docker/
├── Dockerfile.*          # one image per service
├── docker-compose.yml      # Postgres stack
├── docker-compose.sqlite.yml
├── .env.example            # version pins (copy to .env)
├── config/                 # service TOML for containers
├── secrets/                # dev-only RSA keys
├── postgres/init.sql
└── scripts/register-service.sh
```
