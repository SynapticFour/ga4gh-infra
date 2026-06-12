# Template: `bbmri-registered-access-illustrative-v1`

## Source

- BBMRI-ERIC policy (access via biobank negotiation): https://www.healthinformationportal.eu/health-information-sources/bbmri-eric-directory (references DOI 10.5281/zenodo.1241061)
- BBMRI Directory scripts (DUO normalization in QC tooling): https://github.com/BBMRI-ERIC/directory-scripts
- BBMRI Federated Platform data protection concept (bilateral agreements): https://www.bbmri-eric.eu/wp-content/uploads/BBMRI_ERIC_Federated_Search_Platform_Data_Protection_Concept_1-9.pdf

## DUO codes chosen

| Code | Role | Why |
|------|------|-----|
| `GRU` (required) | Permission | Illustrative registered-access research scope |
| `NPU` (required) | Modifier | Common non-profit framing in European biobank contexts |
| `IRB`, `COL` (allowed) | Modifiers | Typical procedural add-ons |

## Why illustrative

BBMRI-ERIC documents **bilateral** sample/data agreements between requesters and biobanks. Public sources confirm DUO tagging in the Directory QC pipeline, but **no single published DUO code set** represents all BBMRI collections. This template is a **strawman** for federated registered access — not a verbatim BBMRI legal agreement.

## Conditions DUO cannot capture

- Biobank-specific MTA/DTA text
- National GDPR legal bases and transfer mechanisms
- BBMRI AAI authentication and Acceptable Use Policy acceptance

## `is_illustrative`

**true** — explicitly marked in seed JSON and here.
