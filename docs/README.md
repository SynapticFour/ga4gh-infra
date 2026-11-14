# ga4gh-infra documentation

Every document under `docs/`, grouped as a tutorial, how-to guides, reference, and explanation. One line each.

ga4gh-infra brokers an institute sign-in. It does not replace that sign-in. Docker Compose is the supported path. A run is not GA4GH certification.

## Tutorial

| Document | What it is |
|----------|------------|
| [GETTING-STARTED.md](GETTING-STARTED.md) | Docker Compose, the native installer, and the Raspberry Pi assets. |

## How-to guides

| Document | What it is |
|----------|------------|
| [AFRICA-DEPLOYMENT.md](AFRICA-DEPLOYMENT.md) | All-in-one profile for an offline field board. |
| [admin-ui/configuration.md](admin-ui/configuration.md) | Admin UI configuration |
| [admin-ui/overview.md](admin-ui/overview.md) | Admin UI (Phase 9) |
| [admin-ui/roles.md](admin-ui/roles.md) | Admin UI roles |
| [agreement-registry/implementation-guide.md](agreement-registry/implementation-guide.md) | Implementation guide — from consent text to PolicyProfile |
| [contributing.md](contributing.md) | Development, testing, and pull requests. |
| [deployment-scenarios.md](deployment-scenarios.md) | Desktop, institute, Raspberry Pi, and federation notes. |
| [idp-integration.md](idp-integration.md) | Keycloak, Entra ID, ELIXIR LS Login, and Shibboleth OIDC. |
| [key-rotation.md](key-rotation.md) | RS256 overlap runbook. |
| [production-deployment.md](production-deployment.md) | TLS, Keycloak, secrets, Postgres, and a go-live checklist. |

## Reference

| Document | What it is |
|----------|------------|
| [CI.md](CI.md) | Which workflows run, and which start only when someone asks. |
| [FOR-EVALUATORS.md](FOR-EVALUATORS.md) | What is tested, the Compose limit, and the 0.1.0 / tag split. |
| [PASSPORT-SIZE.md](PASSPORT-SIZE.md) | How large a Passport may get. |
| [THREAT_MODEL.md](THREAT_MODEL.md) | Threat notes for the identity plane. |
| [ads/README.md](ads/README.md) | Access Decision Service (ADS) |
| [ads/database-schema.md](ads/database-schema.md) | ADS database schema |
| [ads/domain-model.md](ads/domain-model.md) | ADS domain model |
| [ads/events.md](ads/events.md) | ADS event model |
| [ads/integration.md](ads/integration.md) | ADS integration guide |
| [ads/openapi.yaml](ads/openapi.yaml) | OpenAPI description of the access decision service. |
| [ads/sequence-diagrams.md](ads/sequence-diagrams.md) | ADS sequence diagrams |
| [agreement-registry/README.md](agreement-registry/README.md) | Agreement registry documentation |
| [agreement-registry/limitations.md](agreement-registry/limitations.md) | Agreement registry — limitations |
| [agreement-registry/templates/bbmri-registered-access-illustrative-v1.md](agreement-registry/templates/bbmri-registered-access-illustrative-v1.md) | Template: `bbmri-registered-access-illustrative-v1` |
| [agreement-registry/templates/duos-dbgap-gru-ncu-v1.md](agreement-registry/templates/duos-dbgap-gru-ncu-v1.md) | Template: `duos-dbgap-gru-ncu-v1` |
| [agreement-registry/templates/ega-disease-specific-mondo-v1.md](agreement-registry/templates/ega-disease-specific-mondo-v1.md) | Template: `ega-disease-specific-mondo-v1` |
| [agreement-registry/templates/ega-general-research-use-v1.md](agreement-registry/templates/ega-general-research-use-v1.md) | Template: `ega-general-research-use-v1` |
| [agreement-registry/templates/ega-health-medical-biomedical-v1.md](agreement-registry/templates/ega-health-medical-biomedical-v1.md) | Template: `ega-health-medical-biomedical-v1` |
| [configuration.md](configuration.md) | Config reference for the services. |
| [ga4gh-compliance.md](ga4gh-compliance.md) | AAI, Passport, and Service Info self-assessment. Not a certificate. |
| [limitations.md](limitations.md) | What this repository does not do. |
| [security.md](security.md) | Security contract for a deployment. |
| [versioning.md](versioning.md) | Releases, semver, Docker tags, and ARM binaries. |

## Explanation

| Document | What it is |
|----------|------------|
| [ARCHITECTURE.md](ARCHITECTURE.md) | Broker as a relying party, services, and the Passport flow. Compose is the supported path. |
| [DECISIONS.md](DECISIONS.md) | Decisions for the Africa profile and the Ferrum boundary. |
| [ECOSYSTEM.md](ECOSYSTEM.md) | How this repository sits beside Ferrum and HelixTest. |
| [IDENTITY.md](IDENTITY.md) | Who the plane is for, and the 0.1.0 / stack-tag split. |
| [README.md](README.md) | This index. |
| [ads/architecture.md](ads/architecture.md) | ADS architecture |
| [agreement-registry/architecture.md](agreement-registry/architecture.md) | Agreement registry architecture |
| [agreement-registry/policy-background.md](agreement-registry/policy-background.md) | Policy background — GA4GH Framework, MRCG, DUO, DACReS |
| [design-decisions.md](design-decisions.md) | Decision log for the broker. |
| [governance.md](governance.md) | How a signed release is cut. |
| [roadmap.md](roadmap.md) | Suggested phases. Not a calendar. |

Files outside `docs/` that the same reading needs: [CHANGELOG.md](../CHANGELOG.md), [SECURITY.md](../SECURITY.md), [RELEASING.md](../RELEASING.md), [LICENSE](../LICENSE), [NOTICE](../NOTICE), [CITATION.cff](../CITATION.cff).
