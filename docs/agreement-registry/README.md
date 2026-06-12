# Agreement registry documentation

Optional GA4GH policy-to-DUO compatibility layer (Phase 8).

| Document | Description |
|----------|-------------|
| [architecture.md](architecture.md) | Four-level model, extension points for Ferrum / DAC tooling |
| [policy-background.md](policy-background.md) | GA4GH Framework, MRCG, DUO, DACReS summary |
| [implementation-guide.md](implementation-guide.md) | Consent text → `PolicyProfile` workflow |
| [limitations.md](limitations.md) | Honest scope boundaries |
| [templates/](templates/) | One doc per seed `AgreementTemplate` with citations |

Seed JSON lives in `crates/agreement-registry/seeds/`. Types and matching logic live in `ga4gh-types` (`agreement`, `compatibility` modules).

**HTTP REST service:** not implemented in this review checkpoint — see [architecture.md](architecture.md).
