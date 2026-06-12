# Template: `duos-dbgap-gru-ncu-v1`

## Source

- Broad Institute DUOS: https://duos.broadinstitute.org/
- DUOS empirical validation paper: https://pmc.ncbi.nlm.nih.gov/articles/PMC9903839/
- NIH DUO alignment (dbGaP consent groups): https://pmc.ncbi.nlm.nih.gov/articles/PMC10504671/

## DUO codes chosen

| Code | Role | Why |
|------|------|-----|
| `GRU` (required) | Permission | Dominant dbGaP permission in NIH mapping (~1,400 consent groups) |
| `NCU` (required) | Modifier | Common commercial-use restriction structured in DUOS trials |
| `PUB`, `COL`, `IRB` (allowed) | Modifiers | Typical dbGaP add-on restrictions |

## Consent mapping (MRCG)

- *“General research use”* → `GRU`
- *“Non-commercial use only”* → `NCU`

## Conditions DUO cannot capture

- NIH institutional signing official (“Library Card”) pre-authorization
- dbGaP project-specific restrictions not encoded in DUO

## `is_illustrative`

**false** — reflects DUOS production use and published dbGaP DUO structuring rates (~96% mappable).
