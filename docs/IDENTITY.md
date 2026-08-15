# Who ga4gh-infra is for

ga4gh-infra is a **self-hostable identity plane**: OIDC broker, Passport/Visa issuance, ADS, DUO, service registry.

It is a complete product without Ferrum. `make up` brings the stack. Ferrum is one consumer among others.

## Audience

AAI operators, federation architects, institutes that already have (or want) GA4GH Passports independent of a particular data platform.

**Not for:** storing BAM/CRAM (Ferrum), clinical consent (Solum), a researcher UI (BRA).

## Standalone

```bash
git clone https://github.com/SynapticFour/ga4gh-infra.git && cd ga4gh-infra
make prove       # workspace tests, no Docker
make up          # local identity stack (generates gitignored PEMs)
```

Compose defaults **generate** RSA PEMs under `docker/secrets/` (gitignored). Laptop `docker-compose.yml` hardcodes **dev-only** passwords and API keys (`ga4gh` / `dev-*-api-key`). Do not copy that file to a network. Production: [docker-compose.prod.example.yml](../docker/docker-compose.prod.example.yml) with env-injected secrets.

## License (open-core)

This repo is **Apache-2.0**. That is intentional: an institute can run Passports/DUO/ADS without buying a proprietary identity stack, and can stand this plane **against** Ferrum (or another implementer). What Synaptic Four sells is Ferrum / Solum / BRA licenses and optional support — not a closed ga4gh-infra SKU.

## Optional composition

| Join | What you gain | Contract |
|------|----------------|----------|
| Ferrum | Gateway uses clearinghouse + external auth | git tag `ga4gh-infra-v*` in Ferrum `VERSIONS.lock` |
| HelixTest | Passport-on-DRS checks | `helixtest --mode ferrum+infra --profile ferrum-infra` |

See [ECOSYSTEM.md](ECOSYSTEM.md).
