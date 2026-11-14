# For evaluators

Factual snapshot of this repository. Not a sales brief. Not legal advice.

## Maturity

**Early use** on the Compose path. Registry: GA4GH-INFRA-MATURITY-001. The claim's plain text says beta. The public label for that stage is Early use. Only the Docker Compose path is labelled this way. Docker Compose (`make up` / `make up-local`) is the supported deploy. Helm under `deploy/k8s/chart` is labelled **SKETCH** in that chart README.

One maintainer organisation. **No** independent third-party security audit ([limitations.md](limitations.md)). Not GA4GH certification.

Crate versions in this workspace are **0.1.0**. The stack tag `ga4gh-infra-v0.2.8` peels to `4e2f60488bdcc5aea909bbc2238000e49431d076`. The default branch this page was written from is `97a8a61992b20d8a6fc48edf1739d8c5fa01ab05`, twelve commits after that tag. The tag is not this commit.

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
