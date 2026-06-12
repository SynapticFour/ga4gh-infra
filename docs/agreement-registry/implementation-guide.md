# Implementation guide — from consent text to PolicyProfile

This guide helps an institution translate **consent form language** into a machine-readable **`PolicyProfile`**, following the GA4GH **Machine Readable Consent Guidance (MRCG)** approach.

## Prerequisites

- Institutional consent form or data access policy (human text)
- Identified dataset or cohort owner id
- DUO term reference (`duo-service` `/terms` or [EBISPOT/DUO](https://github.com/EBISPOT/DUO))
- Optional: link to legal record (`source_document_ref` — e.g. Ferrum DTA id)

## Step 1 — Extract data-use permissions from consent

Read the consent section that limits **what research is allowed**. Map to a DUO **permission** code:

| Consent language (examples) | DUO permission |
|----------------------------|----------------|
| General research use | `GRU` |
| Health / medical / biomedical research only | `HMB` |
| Specific disease only | `DS` (+ MONDO term) |
| Population origins / ancestry only | `POA` |
| Genetic studies only | `GSO` |
| No restriction | `NRES` |

Record rationale in `DuoCodeAssertion.rationale` for DAC audit.

## Step 2 — Extract modifiers and restrictions

Map additional bullets to DUO **modifiers**:

| Consent language (examples) | DUO modifier |
|----------------------------|--------------|
| Non-profit use only | `NPU` |
| Non-commercial use only | `NCU` |
| Publication required | `PUB` |
| Collaboration required | `COL` |
| Ethics approval required | `IRB` |
| Geographic restriction | `GS` (+ location in rationale) |

If consent says “disease X only”, use `DS` with `modifier_value: "MONDO:..."` per [EGA guidance](https://dac.ega-archive.org/take-the-tour/policies/create).

## Step 3 — Choose or define an AgreementTemplate

- Prefer a **seed template** from [templates/](templates/) if it matches your policy pattern.
- Set `based_on_template` on the dataset profile to that template id.
- If no template fits, register a custom template (future HTTP API) or use a profile without a template.

## Step 4 — Build the PolicyProfile JSON

Example (researcher-side profile aligned with DUOS GRU+NCU):

```json
{
  "id": "profile.researcher.lab-alpha",
  "owner": "institution:lab-alpha",
  "duo_codes": [
    { "code": "GRU", "rationale": "General research use per institutional IRB" },
    { "code": "NCU", "rationale": "Non-commercial use only — no industry funding on this project" }
  ],
  "based_on_template": "duos-dbgap-gru-ncu-v1",
  "version": "1.0.0",
  "effective_date": "2024-06-01T00:00:00Z",
  "source_document_ref": "ferrum:dta:example-opaque-id",
  "visa_types": ["ControlledAccessGrants", "ResearcherStatus"]
}
```

## Step 5 — Run compatibility check (library today; HTTP later)

```rust
use ga4gh_types::{check_compatibility, PolicyProfile, /* ... */};

let result = check_compatibility(&requester_profile, &dataset_profile, Some(&template));
// Inspect result.compatible, result.unsatisfied_codes, result.conditions
```

Treat `conditions` as **required human follow-up** even when `compatible: true`.

## Step 6 — Institutional sign-off

- Legal counsel validates the DUO mapping against consent text.
- DAC records the `decision_record_id` in their application system (level 4).
- Access is granted only after DAC approval — not by this component alone.

See [limitations.md](limitations.md).
