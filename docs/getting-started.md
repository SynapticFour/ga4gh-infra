# Getting started

Two single-command paths to run GA4GH Infra locally.

## Path 1: Docker (recommended)

**Prerequisites:** Docker, Docker Compose, optional [just](https://github.com/casey/just)

```bash
git clone https://github.com/SynapticFour/ga4gh-infra.git
cd ga4gh-infra
just up
```

What happens:

1. Dev RSA keys are created under `docker/secrets/` if missing (`ga4gh-infra keygen`).
2. Compose builds per-service images and starts PostgreSQL, mock IdP, broker, visa-registry, duo-service, service-registry, and sample-resource.
3. Health checks wait until all services respond on ports 8080–8084 and 9000.

Lighter stack (SQLite visa-registry, no Postgres for visas):

```bash
just up-sqlite
```

Stop and view logs:

```bash
just down
just logs
```

Run the full integration test:

```bash
just e2e
# or: ./scripts/e2e.sh
```

### Service URLs (default)

| Service | URL |
|---------|-----|
| AAI broker | http://localhost:8080 |
| Visa registry | http://localhost:8081 |
| DUO service | http://localhost:8082 |
| Service registry | http://localhost:8083 |
| Sample resource | http://localhost:8084 |
| Mock IdP | http://localhost:9000 |

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

**Note:** Service-registry still expects PostgreSQL at `SERVICE_REGISTRY_DATABASE_URL`. For a fully containerized demo without local Postgres, use `just up-sqlite` instead.

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
- [configuration.md](configuration.md) — all config fields
- [deployment-scenarios.md](deployment-scenarios.md) — desktop, institute production, federation notes
- [limitations.md](limitations.md) — what this project does not do
