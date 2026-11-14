# ga4gh-infra

ga4gh-infra connects an institute's own sign-in to GA4GH Passports, visas, and access decisions. It does not replace that sign-in.

For an institute that already runs its own sign-in and wants those GA4GH tokens from a service it hosts.

**Status: Early use.** Compose path. Registry: GA4GH-INFRA-MATURITY-001. Early use is the public label for the stage the claim calls beta. Only the Docker Compose path is labelled this way.

[![CI](https://github.com/SynapticFour/ga4gh-infra/actions/workflows/ci.yml/badge.svg)](https://github.com/SynapticFour/ga4gh-infra/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/SynapticFour/ga4gh-infra/graph/badge.svg)](https://codecov.io/gh/SynapticFour/ga4gh-infra)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

GA4GH is a set of shared interfaces for genomic and health data. A Passport is a token that says who may access what. This repository brokers the institute's sign-in and issues that token. It is not the identity provider. [HelixTest](https://github.com/SynapticFour/HelixTest) checks a running API and does not issue Passports. [HELIOS](https://github.com/SynapticFour/HELIOS) signs a file already on disk.

## What it does

- It brokers OpenID Connect sign-in, issues Passports and visas, decides access, matches DUO terms, and keeps a service registry. Registry: TOOL-GA4GH-INFRA-USE-001.
- The broker is a relying party. Researchers sign in at the institute. `mock-idp` is only a test identity provider.
- `make prove` runs the workspace tests. It does not need Docker.
- `make up` starts the Compose stack. That is the supported path.
- Crate versions in this workspace are 0.1.0. The stack tag is `ga4gh-infra-v0.2.8`. Those are different numbers on purpose. Image labels in `docker/.env.example` use 0.2.8. Registry: WEB-VERSION-002.

## Limits

- Early use names the Compose path only. Helm under `deploy/k8s/chart` is a sketch. Registry: GA4GH-INFRA-MATURITY-001.
- The tag `ga4gh-infra-v0.2.8` peels to `4e2f60488bdcc5aea909bbc2238000e49431d076`. This default branch is `97a8a61992b20d8a6fc48edf1739d8c5fa01ab05`, twelve commits after that tag. The tag is not this commit.
- There is no third-party security audit. A green library test is not GA4GH certification. Registry: TOOL-GA4GH-INFRA-STATUS-001.
- The library check and the Compose job start when someone asks. They do not run on every push.
- Passport and visa flows may involve identifiable researcher data. This page is not legal advice. See [docs/limitations.md](docs/limitations.md).
- A test sign-in in the source does not replace an institute sign-in.

## Quick start

Prerequisites for `make prove`: Rust 1.91.1, pinned in `rust-toolchain.toml`.

```bash
make prove
```

You should see `ga4gh-infra prove OK. Live stack: make up`.

`make up` needs Docker Compose. It builds images and waits until health checks pass. You should see the admin UI URL `http://localhost:8095`. This repository starts that stack. It does not start Ferrum. The walkthrough is [docs/GETTING-STARTED.md](docs/GETTING-STARTED.md).

## Documentation

Every file under `docs/` is listed in [docs/README.md](docs/README.md). The groups are a tutorial, how-to guides, reference, and explanation.

### Tutorial

- [Getting started](docs/GETTING-STARTED.md)

### How-to guides

- [Deployment scenarios](docs/deployment-scenarios.md)
- [Production deployment](docs/production-deployment.md)
- [Identity provider integration](docs/idp-integration.md)
- Further task pages are in the [index](docs/README.md#how-to-guides)

### Reference

- [For evaluators](docs/FOR-EVALUATORS.md)
- [Limitations](docs/limitations.md)
- [Configuration](docs/configuration.md)
- [Versioning](docs/versioning.md)
- Further reference pages are in the [index](docs/README.md#reference)

### Explanation

- [Architecture](docs/ARCHITECTURE.md)
- [Decisions](docs/DECISIONS.md)
- [Design decisions](docs/design-decisions.md)

## Licence

This tool is free to use under the Apache License 2.0, and you can read the source. Registry: TOOL-GA4GH-INFRA-LICENSE-001. Not legal advice. The file is [LICENSE](LICENSE). See also [NOTICE](NOTICE).

## Cite

Use [CITATION.cff](CITATION.cff). Prefer that file over a hand-copied line.

## Website

- Open tools: [https://synapticfour.com/en/offene-werkzeuge/](https://synapticfour.com/en/offene-werkzeuge/)
- Technical profile: [https://synapticfour.com/en/technical/code/](https://synapticfour.com/en/technical/code/)

The same pages exist in German (`/de/offene-werkzeuge/`, `/de/technik/code/`) and in French (`/fr/offene-werkzeuge/`, `/fr/technique/code/`).

The map of repositories: [profile/README.md](https://github.com/SynapticFour/.github/blob/main/profile/README.md).

Contact: [contact@synapticfour.com](mailto:contact@synapticfour.com)
