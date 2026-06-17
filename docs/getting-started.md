# Getting started

Two single-command paths to run GA4GH Infra locally.

## Path 1: Docker (recommended)

**Prerequisites:** Docker, Docker Compose, optional [just](https://github.com/casey/just) or GNU Make

```bash
git clone https://github.com/SynapticFour/ga4gh-infra.git
cd ga4gh-infra
make up-local
# equivalent: just up
```

Use a **hyphen** — `make up-local`, not `make up local`.

What happens:

1. Dev RSA keys are created under `docker/secrets/` if missing (`ga4gh-infra keygen`).
2. Rust crates are vendored for offline Docker builds (`make prepare-vendor`).
3. Compose builds per-service images and starts PostgreSQL, mock IdP, broker, visa-registry, access-decision-service, duo-service, service-registry, agreement-registry, admin-ui, and sample-resource.
4. Health checks wait until services respond.
5. Demo seed data is loaded automatically (`make seed`) — datasets, a pending DAC request, and operator login users.

Lighter stack (SQLite visa-registry, no Postgres for visas):

```bash
make up-sqlite
# or: just up-sqlite
```

Stop and view logs:

```bash
make down
make logs
```

Re-load demo data without restarting:

```bash
make seed
```

Run the full integration test:

```bash
make test
# or: just e2e / ./scripts/e2e.sh
```

### Service URLs (default)

| Service | URL |
|---------|-----|
| AAI broker | http://localhost:8080 |
| Visa registry | http://localhost:8081 |
| DUO service | http://localhost:8082 |
| Service registry | http://localhost:8083 |
| Sample resource | http://localhost:8084 |
| Agreement registry | http://localhost:8086 |
| Access Decision Service | http://localhost:8090 |
| **Admin UI** | **http://localhost:8095** |
| Mock IdP | http://localhost:9000 |

### Admin UI quick try

1. Open http://localhost:8095
2. Log in via mock IdP (e.g. `researcher@uni-heidelberg.de` — Operator role)
3. Review **DAC Queue** — seed data includes a pending request; approve or reject with htmx row updates
4. Browse datasets, grants, audit log, and dashboard activity feed

See [admin-ui/overview.md](admin-ui/overview.md) for roles and pages.

---

## Path 2: Native binary (all-in-one)

**Prerequisites:** curl, tar — no Rust toolchain required

### Linux / macOS

```bash
curl -sSL https://raw.githubusercontent.com/SynapticFour/ga4gh-infra/main/scripts/install.sh | sh
```

### Windows

```powershell
irm https://raw.githubusercontent.com/SynapticFour/ga4gh-infra/main/scripts/install.ps1 | iex
```

What happens:

1. The script detects your OS/CPU and downloads the matching `ga4gh-infra` release binary.
2. Config is written to `~/.config/ga4gh-infra/` (SQLite for visa-registry).
3. RS256 signing keys are generated under `secrets/` when missing.
4. An `env` file is created with bootstrap secrets and database URLs.

Start the stack:

```bash
source ~/.config/ga4gh-infra/env   # edit secrets first
ga4gh-infra all-in-one --config ~/.config/ga4gh-infra/all-in-one.toml
```

**Note:** Service-registry still expects PostgreSQL at `SERVICE_REGISTRY_DATABASE_URL`. For a fully containerized demo without local Postgres, use `make up-sqlite` instead.

---

## Raspberry Pi (ARM)

Prebuilt release binaries include ARM Linux targets:

| Raspberry Pi OS | `uname -m` | Release asset |
|-----------------|------------|---------------|
| 64-bit (Pi 4/5 recommended) | `aarch64` | `ga4gh-infra-aarch64-unknown-linux-gnu.tar.gz` |
| 32-bit | `armv7l` | `ga4gh-infra-armv7-unknown-linux-gnueabihf.tar.gz` |

On the Pi:

```bash
curl -sSL https://raw.githubusercontent.com/SynapticFour/ga4gh-infra/main/scripts/install.sh | sh
```

The install script selects the correct asset automatically. Prefer **64-bit Raspberry Pi OS** for best performance.

For container deployment on Pi, use Docker with `linux/arm64` or `linux/arm/v7` platform images when available (see [deployment-scenarios.md](deployment-scenarios.md)).

---

## Next steps

- [architecture.md](architecture.md) — how components interact
- [admin-ui/overview.md](admin-ui/overview.md) — operations dashboard
- [configuration.md](configuration.md) — all config fields
- [deployment-scenarios.md](deployment-scenarios.md) — desktop, institute production, federation notes
- [limitations.md](limitations.md) — what this project does not do
