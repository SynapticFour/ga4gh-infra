# Who ga4gh-infra is for

ga4gh-infra is a **self-hostable identity plane**: an OIDC broker, Passport/Visa issuance, ADS, DUO, and a service registry. The broker is an OIDC **relying party**. It is not an identity provider and it is not an OIDC authorization server. Researchers authenticate at an institute IdP. `mock-idp` is a test identity provider for Docker and e2e, not the broker.

It is a complete product without Ferrum. `make up` brings the stack. Ferrum is one consumer among others.

## Audience

AAI operators, federation architects, institutes that already have (or want) GA4GH Passports independent of a particular data platform.

**Not for:** storing BAM/CRAM (Ferrum), clinical consent (Solum), a researcher UI (BRA).

## Standalone

```bash
git clone https://github.com/SynapticFour/ga4gh-infra.git && cd ga4gh-infra
make prove       # workspace tests, no Docker
make up          # local stack; broker is an OIDC relying party (mock-idp is the test IdP)
```

Compose defaults **generate** RSA PEMs under `docker/secrets/` (gitignored). Laptop `docker-compose.yml` hardcodes **dev-only** passwords and API keys (`ga4gh` / `dev-*-api-key`). Do not copy that file to a network. Production: [docker-compose.prod.example.yml](../docker/docker-compose.prod.example.yml) with env-injected secrets.

## License (open-core)

This repo is **Apache-2.0**. That is intentional: an institute can run this relying-party broker, Passports, DUO, and ADS without buying a closed ga4gh-infra SKU. The broker does not replace the institute identity provider. What Synaptic Four sells is Ferrum / Solum / BRA licenses and optional support.

Crate `Cargo.toml` versions stay **0.1.0**; the product/stack SemVer is the git tag **`ga4gh-infra-v*`**. Image labels in `docker/.env.example` follow that tag (checked 2026-10-14: `ga4gh-infra-v0.2.8`). See [versioning.md](versioning.md). That split is deliberate, not drift.

## Optional composition

| Join | What you gain | Contract |
|------|----------------|----------|
| Ferrum | Gateway uses clearinghouse + external auth | git tag `ga4gh-infra-v*` in Ferrum `VERSIONS.lock` |
| HelixTest | Passport-on-DRS checks | `helixtest --mode ferrum+infra --profile ferrum-infra` |

See [ECOSYSTEM.md](ECOSYSTEM.md).
