# Agreement registry architecture

The **agreement-registry** component operationalizes GA4GH policy products at the **policy-to-DUO** and **institution-compatibility** layers. It does not replace legal agreements, DAC meetings, or jurisdiction-specific regulation.

## Four-level model

| Level | What it covers | Implemented here? |
|-------|----------------|-------------------|
| **1 — Legal / DTA text** | Material transfer agreements, consent form prose, jurisdictional clauses | **No** — extension point only |
| **2 — Policy → DUO translation** | MRCG worksheet: consent language → DUO code sets | **Yes** — `PolicyProfile`, `AgreementTemplate` |
| **3 — Technical compatibility** | Compare requester vs dataset DUO profiles | **Yes** — `check_compatibility()` |
| **4 — Operational DAC workflow** | Applications, committee review, signing | **No** — extension point only |

Related SynapticFour work at level 1 (e.g. [Ferrum](https://github.com/SynapticFour/Ferrum)) is intentionally **out of scope** for this crate. Integration is via opaque references, not document parsing.

## Extension points

### Level 1 — Legal / DTA systems (e.g. Ferrum)

| Seam | Type | Direction |
|------|------|-----------|
| `PolicyProfile.source_document_ref` | Opaque string | External system → agreement-registry |
| `AgreementTemplate.reference_url` | HTTPS URL | Human-readable agreement text |

**Not implemented:** fetching or validating legal text, versioning DTAs, or syncing Ferrum state. Operators copy DUO mappings from legal review into `PolicyProfile.duo_codes` manually or via future adapters.

### Level 4 — DAC operational tooling

| Seam | Type | Direction |
|------|------|-----------|
| `CompatibilityCheckResult.decision_record_id` | Stable id | agreement-registry → DAC audit exports |
| `DecisionRecord` | Audit row | agreement-registry → institutional reporting |
| `CompatibilityCheckResult.conditions` | Human strings | agreement-registry → DAC queue items |

**Not implemented:** workflow engines, e-signatures, or automatic access grants. A compatible DUO check is **necessary but not sufficient** for access.

## Relationship to existing services

```text
                    ┌─────────────────────┐
                    │ agreement-registry  │  optional
                    │  PolicyProfile      │
                    │  AgreementTemplate  │
                    │  /compatibility-check│
                    └─────────┬───────────┘
                              │ DUO code sets (not a hard dependency)
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
        duo-service     visa-registry    clearinghouse
        /match terms    visa types       passport policy
```

- **duo-service** — term-level `/match` (dataset codes vs intended use). Agreement-registry compares **whole policy profiles** and templates.
- **visa-registry** — issues signed visa JWTs. Profiles may list `visa_types` for template checks; visas are not stored in agreement-registry.
- **aai-broker / clearinghouse** — no crate dependency on agreement-registry.

## Current implementation status (Phase 8 checkpoint)

| Deliverable | Status |
|-------------|--------|
| `ga4gh-types::agreement` data model | Done |
| `ga4gh-types::compatibility` matching | Done |
| Seed templates + docs | Done |
| `agreement-registry` library + in-memory registry | Done |
| REST HTTP service (`POST /policy-profiles`, etc.) | **Not yet** — next review step |
| Docker / `ga4gh-infra-cli` integration | **Not yet** |

## Data flow (target state)

1. Institution registers `AgreementTemplate` (curated) or references a seed template.
2. Dataset owner registers `PolicyProfile` with `based_on_template` + `duo_codes`.
3. Researcher (or broker on their behalf) registers requester `PolicyProfile`.
4. Client calls `POST /compatibility-check` → `CompatibilityCheckResult` + persisted `DecisionRecord`.
5. DAC / access system uses `conditions` and external legal tools for final approval.

See [implementation-guide.md](implementation-guide.md) and [limitations.md](limitations.md).
