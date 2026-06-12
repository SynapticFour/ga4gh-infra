# Template: `ega-health-medical-biomedical-v1`

## Source

- NIH / dbGaP alignment to DUO: https://pmc.ncbi.nlm.nih.gov/articles/PMC10504671/
- EGA DUO training material: https://www.ebi.ac.uk/training/online/courses/ega-quick-tour/.../data-use-ontology-duo-codes-at-ega/

## DUO codes chosen

| Code | Role | Why |
|------|------|-----|
| `HMB` (required) | Permission | ~800 NIH consent groups mapped to HMB in the DUO alignment study |
| `NPU`, `NCU`, `PUB`, `IRB` (allowed) | Modifiers | Frequently co-occur with HMB in archive policies |

## Consent mapping (MRCG)

Typical consent phrase: *“health, medical, or biomedical research only”* → **`HMB`**.

## Conditions DUO cannot capture

- Industry vs academic eligibility beyond `NCU`/`NPU` wording
- Study-specific collaboration agreements (`COL` modifier still needs named collaborators)

## `is_illustrative`

**false** — grounded in published NIH/dbGaP ↔ DUO mapping statistics and EGA usage.
