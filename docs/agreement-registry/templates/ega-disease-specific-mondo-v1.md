# Template: `ega-disease-specific-mondo-v1`

## Source

- EGA DUO page (DS + MONDO requirement): https://ega-archive.org/access/data-access-committee/data-use-ontology/
- EGA DAC Portal policy tour: https://dac.ega-archive.org/take-the-tour/policies/create

## DUO codes chosen

| Code | Role | Why |
|------|------|-----|
| `DS` (required) | Permission | Disease-specific research permission |
| `RS` (allowed) | Modifier | Research-specific restrictions often paired with DS in archive policies |

## Consent mapping (MRCG)

Typical consent phrase: *“research into [disease X] only”* → **`DS`** with `modifier_value: "MONDO:..."`.

Example from EGA docs: juvenile idiopathic arthritis → `DUO:0000007; MONDO:0011429`.

## Conditions DUO cannot capture

- **MONDO term must be supplied** — compatibility check adds a condition if `modifier_value` is missing
- Clinician verification that proposed use matches the named disease scope
- DAC review of secondary-use proposals outside the named condition

## `is_illustrative`

**false** for the DS+MONDO **pattern** (EGA requirement). Individual disease ids are institution-specific.
