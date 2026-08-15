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

Compose defaults **generate** RSA PEMs under `docker/secrets/` (gitignored). Rotate / replace before any network that is not a laptop. Historical committed keys are public. See [docker/secrets/README.md](../docker/secrets/README.md) and [key-rotation.md](key-rotation.md).

## Optional composition

| Join | What you gain | Contract |
|------|----------------|----------|
| Ferrum | Gateway uses clearinghouse + external auth | git tag `ga4gh-infra-v*` in Ferrum `VERSIONS.lock` |
| HelixTest | Passport-on-DRS checks | `helixtest --mode ferrum+infra --profile ferrum-infra` |

See [ECOSYSTEM.md](ECOSYSTEM.md).
