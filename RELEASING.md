# Releasing ga4gh-infra

Stack/deploy tags are `ga4gh-infra-vX.Y.Z` (operator-facing). Crate `version` in Cargo.toml may stay `0.1.0` — see [docs/versioning.md](docs/versioning.md) and [docs/IDENTITY.md](docs/IDENTITY.md).

## Before tagging

Run `make verify-release` on the commit you will tag. That is the fmt, clippy, test, and cargo-audit gate. GitHub product CI does not run on push to `main` or on pull requests. Dispatch [`.github/workflows/ci.yml`](.github/workflows/ci.yml) when you also want Docker e2e, coverage, or ARM. Triggers and rough minutes: [docs/CI.md](docs/CI.md).

## Release train (portfolio)

When **this** repo is tagged (`ga4gh-infra-v*`): the same week, bump Ferrum `VERSIONS.lock` (`GA4GH_INFRA_REF` / `SHA`) and Ferrum-GA4GH-Demo `GA4GH-INFRA-git`. Showcase pins **tags that exist on origin/main**.

When **Ferrum** is tagged: Lab Kit and Ferrum-GA4GH-Demo follow that week (this identity plane does not have to retag).

See [Ferrum PORTFOLIO.md](https://github.com/SynapticFour/Ferrum/blob/main/docs/PORTFOLIO.md).
