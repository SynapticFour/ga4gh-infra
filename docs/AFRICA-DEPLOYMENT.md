# Africa Deployment Guide (ga4gh-infra)

Resource-constrained and offline-first deployment profile for the GA4GH **identity and access plane** — complementary to [Ferrum Africa/Laptop Mode](https://github.com/SynapticFour/Ferrum/blob/main/docs/AFRICA-DEPLOYMENT.md) (data plane).

## What Africa-Mode provides

| Feature | Description |
|---------|-------------|
| **Zero PostgreSQL** | SQLite for visa-registry, service-registry, and ADS |
| **Single binary** | `ga4gh-infra all-in-one --africa` runs all five services |
| **Embedded mock IdP** | Field labs without an external OIDC provider |
| **Co-deploy ports** | 8180–8190 block avoids clash with Ferrum on 8080 |
| **`GA4GH_OFFLINE=1`** | Shortcut equivalent to Ferrum's `FERRUM_OFFLINE=1` |

## Quick start (Raspberry Pi / laptop)

```bash
curl -sSL https://raw.githubusercontent.com/SynapticFour/ga4gh-infra/main/scripts/install.sh | sh
source ~/.config/ga4gh-infra/env
ga4gh-infra keygen --output-dir ~/.config/ga4gh-infra/secrets
cp config/all-in-one.africa.toml.example ~/.config/ga4gh-infra/all-in-one.toml
# Edit {{CONFIG_DIR}} placeholders or use install.sh output
ga4gh-infra all-in-one --config ~/.config/ga4gh-infra/all-in-one.toml --africa
```

Verify:

```bash
curl http://127.0.0.1:8180/service-info
curl http://127.0.0.1:8183/services
```

## Docker (co-deploy with Ferrum)

From the ga4gh-infra repo:

```bash
docker compose -f docker/docker-compose.sqlite.yml --env-file docker/.env up --build --wait
```

This stack uses SQLite for all registries, includes ADS, and binds to **8180–8190** so Ferrum can use **8080**.

## Configuration

`[africa]` section in all-in-one TOML:

```toml
[africa]
offline_first = true
max_memory_mb = 512
embedded_mock_idp = true
data_dir = "~/.config/ga4gh-infra/data"
jwks_cache_ttl_seconds = 600
```

## Co-deploy with Ferrum

When Ferrum runs with `[auth] mode = "external"` and `[discovery] enabled = true`:

1. Ferrum validates Passports via ga4gh-infra broker JWKS (`ga4gh-clearinghouse`)
2. Ferrum registers DRS/Beacon/WES services in ga4gh-infra service-registry on startup
3. Ferrum resolves peer service URLs from the registry (config fallbacks when offline)

See [Ferrum-Lab-Kit co-deploy profiles](https://github.com/SynapticFour/Ferrum-Lab-Kit) (`field-edge+infra`, `institute`) and [Ferrum-GA4GH-Demo `./run --with-infra`](https://github.com/SynapticFour/Ferrum-GA4GH-Demo).

## Resource guidance

| Profile | RAM | Notes |
|---------|-----|-------|
| Minimum | 512 MB | Broker + SQLite registries only |
| Recommended (Pi 4/5) | 1 GB | All-in-one with embedded mock IdP |
| With Ferrum co-deploy | 4 GB+ | Ferrum laptop + ga4gh-infra africa |

## Preflight

```bash
scripts/africa-preflight.sh
```

Checks: ARM binary, disk space, port availability (8180–8190), SQLite write access.
