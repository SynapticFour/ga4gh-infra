# Template: `ega-general-research-use-v1`

## Source

- EGA Data Use Ontology documentation: https://ega-archive.org/access/data-access-committee/data-use-ontology/
- EGA blog on DUO adoption: https://blog.ega-archive.org/data-use-ontology
- GA4GH DUO standard (production use at EGA scale): https://pubmed.ncbi.nlm.nih.gov/34820659/

## DUO codes chosen

| Code | Role | Why |
|------|------|-----|
| `GRU` (required) | Permission | EGA documents GRU as a primary search/discovery tag for general research datasets |
| `NPU`, `NCU`, `PUB`, `COL`, `IRB` (allowed) | Modifiers | Common EGA DAC policy add-ons documented on the EGA DUO table |

## Consent mapping (MRCG)

Typical consent phrase: *“data may be used for general research purposes subject to DAC approval”* → **`GRU`**.

## Conditions DUO cannot capture

- DAC application and manual approval workflow
- Dataset-specific embargo or moratorium dates (use `MOR` + dates in profile if applicable)
- Cross-border transfer clauses

## `is_illustrative`

**false** — reflects a widely documented EGA tagging pattern, not a single signed bilateral agreement text.
