# CI triggers

Push to `main` and pull requests run only the secret scan and dependency review. Product CI is manual. A tag publishes binaries and images. Run `make verify-release` before tagging. That command is fmt, clippy, workspace tests, the all-features library tests, and `cargo audit` when `cargo-audit` is installed.

Minutes below are rough runner time, not a measured bill.

| Workflow | Trigger | Rough minutes | When to run manually |
|---|---|---|---|
| `ci.yml` | `workflow_dispatch` | 40–90 | Before a tag, after `make verify-release`. One run includes tests, cargo-audit, testcontainers, Docker e2e, coverage, and the ARM build. |
| `secret-scan.yml` | pull request; push to `main` or `master` | 1–2 | Leave it. It stays on those events. |
| `dependency-review.yml` | pull request | 1–2 | Leave it. It stays on pull requests. |
| `release-binaries.yml` | tag `ga4gh-infra-v*`; `workflow_dispatch` | 15–40 | Dispatch only to attach binaries to an existing tag. |
| `docker-release.yml` | tags `aai-broker-v*`, `visa-registry-v*`, `duo-service-v*`, `service-registry-v*`, `access-decision-service-v*`, `ga4gh-infra-v*` | 20–40 | No manual trigger. The tag publishes the images. |
