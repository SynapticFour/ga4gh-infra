# For evaluators

Factual snapshot of this repository. Not a sales brief. Not legal advice.

## Maturity

**Active (beta) — Compose path.** Docker Compose (`make up` / `make up-local`) is the supported deploy. Helm under `deploy/k8s/chart` is labelled **SKETCH** in that chart README — not the supported production path.

One maintainer organisation. **No** independent third-party security audit ([limitations.md](limitations.md)). Not GA4GH certification.

Crate versions in this workspace are **0.1.0**. Published stack tags are `ga4gh-infra-v*`.

## License

Apache License 2.0. See [LICENSE](../LICENSE) and [NOTICE](../NOTICE).

## Tested in this tree

| Claim | Evidence |
|-------|----------|
| Workspace tests | `make prove` (no Docker) |
| Compose stack | `make up` / `make up-local` |
| HelixTest | CI against a running stack (technical signal, not certification) |

## Not tested / not claimed

| Topic | Status |
|-------|--------|
| Helm production | Sketch only |
| SAML2 upstream | Not supported (OIDC only) |
| Third-party audit | None |
| GA4GH certification | Not claimed |
| Combo SKU with Ferrum | Does not exist. Ferrum can use this broker; Ferrum also has built-in passports. |

## Contact

Questions can be sent to [contact@synapticfour.com](mailto:contact@synapticfour.com).
