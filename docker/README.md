# Docker images and Compose stacks

All Docker build contexts live under `docker/`. Build from the **repository root** so `Cargo.toml` and `crates/` are available.

## Dockerfiles

| File | Binary / image | Crate version tag |
|------|----------------|-------------------|
| `Dockerfile.broker` | `aai-broker` | `AAI_BROKER_VERSION` |
| `Dockerfile.visa-registry` | `visa-registry` | `VISA_REGISTRY_VERSION` |
| `Dockerfile.duo-service` | `duo-service` | `DUO_SERVICE_VERSION` |
| `Dockerfile.service-registry` | `service-registry` | `SERVICE_REGISTRY_VERSION` |
| `Dockerfile.access-decision-service` | `access-decision-service` | `ACCESS_DECISION_SERVICE_VERSION` |
| `Dockerfile.all-in-one` | `ga4gh-infra` (combined CLI) | `GA4GH_INFRA_VERSION` |
| `Dockerfile.mock-idp` | `mock-idp` (dev/CI only) | `MOCK_IDP_VERSION` |
| `Dockerfile.sample-resource` | `sample-resource` | `SAMPLE_RESOURCE_VERSION` |
| `Dockerfile.register` | curl helper for one-shot registration | — |

Images use a multi-stage build (`rust:1-bookworm` → `debian:bookworm-slim`) and run as non-root user `ga4gh` (uid 1000). Runtime images include `curl` for Compose health checks.

Example manual build:

```bash
docker build -f docker/Dockerfile.broker -t ghcr.io/<org>/aai-broker:0.2.3 .
```

## Compose stacks

Copy version pins and set your registry prefix:

```bash
cp docker/.env.example docker/.env   # optional local overrides (gitignored)
```

Compose reads `--env-file docker/.env.example` by default in CI; use `docker/.env` locally if you customize pins.

| Compose file | Database | Use case |
|--------------|----------|----------|
| `docker-compose.yml` | PostgreSQL for visa-registry, service-registry, and ADS | Full stack (CI, e2e, dev) |
| `docker-compose.sqlite.yml` | SQLite for visa-registry, service-registry, and ADS (`driver = "sqlite"` in the `*.sqlite.toml` files). No Postgres service in this file. | Lighter local deployment |
| `docker-compose.prod.example.yml` | PostgreSQL; no mock-idp | Production reference (see [docs/production-deployment.md](../docs/production-deployment.md)) |

TLS termination examples: [`reverse-proxy/`](reverse-proxy/README.md).

Start the default stack:

```bash
just up
# or
docker compose -f docker/docker-compose.yml --env-file docker/.env.example up --build --wait
```

Minimal SQLite set, when the rest of the Compose file is not required: aai-broker, visa-registry, and service-registry, using `docker/config/broker.sqlite.toml`, `visa-registry.sqlite.toml`, and `service-registry.sqlite.toml`. Published host ports on the sqlite Compose file are **8180** (broker), **8181** (visa-registry), and **8183** (service-registry). Container ports stay 8080, 8081, and 8083. The sqlite Compose file also starts mock-idp, duo, ADS, sample-resource, agreement-registry, and admin-ui. Those are not part of that three-service set.

RSS for those three processes was not measured here. A planning estimate of about 128 MB per idle process (about 384 MB together, before the OS) is an estimate, not a measurement.

SQLite variant:

```bash
just up-sqlite
# or
docker compose -f docker/docker-compose.sqlite.yml --env-file docker/.env.example up --build --wait
```

### Version pins (`.env`)

Image tags follow the **stack** git tag `ga4gh-infra-v0.2.3` → `:0.2.3` (crate Cargo.toml may stay `0.1.0`). See [docs/versioning.md](../docs/versioning.md):

```env
GA4GH_IMAGE_PREFIX=ghcr.io/synapticfour
AAI_BROKER_VERSION=0.2.3
VISA_REGISTRY_VERSION=0.2.3
# ...
```

Mix versions by editing `docker/.env` before `docker compose up`.

## CI releases

Pushing a git tag triggers `.github/workflows/docker-release.yml`:

| Git tag | Images pushed |
|---------|----------------|
| `ga4gh-infra-v0.2.3` | **every** Compose service + all-in-one as `:0.2.3` (+ `:latest`) |
| `ga4gh-infra-v0.2.2` | all-in-one `:0.2.2` only (workflow predates the stack matrix) |
| `aai-broker-v0.3.0` | `aai-broker:0.3.0` only (mixed-version stacks) |

Replace `<org>` with your GitHub organization or username (lowercase).

## Layout

```text
docker/
├── Dockerfile.*          # one image per service
├── docker-compose.yml      # Postgres stack
├── docker-compose.sqlite.yml
├── .env.example            # version pins (copy to .env)
├── config/                 # service TOML for containers
├── secrets/                # generated locally (gitignored PEMs); README only in git
├── postgres/init.sql
└── scripts/register-service.sh
```
