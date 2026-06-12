# Policy background — GA4GH Framework, MRCG, DUO, DACReS

This document summarizes the **conceptual basis** for agreement-registry. It paraphrases public GA4GH materials; it is not legal advice and does not reproduce source documents at length.

## GA4GH Framework for Responsible Sharing

- **URL:** https://www.ga4gh.org/framework/
- **Role here:** Background principles (responsible sharing, security, privacy, accountability). The Framework is **not machine-readable** and is **not implemented in code**.
- **Use in this project:** Context for why DUO + audit records exist; operators still perform ethical and legal review outside this service.

## Machine Readable Consent Guidance (MRCG)

- **URL:** https://www.ga4gh.org/wp-content/uploads/Machine-readable-Consent-Guidance_6JUL2020-1.pdf
- **Role here:** Primary translation pattern — map consent form language to structured data-use terms (DUO).
- **Use in this project:** `PolicyProfile.duo_codes` with optional `rationale` fields mirrors an operationalized MRCG worksheet. [implementation-guide.md](implementation-guide.md) walks through this mapping.

## Data Use Ontology (DUO)

- **URL:** https://github.com/EBISPOT/DUO
- **Role here:** Shared vocabulary already used by `duo-service` and major archives (EGA, Broad DUOS).
- **Use in this project:** Agreement templates and profiles are **sets of DUO codes** (permissions + modifiers), not redefinitions of DUO terms.

## DACReS (Data Access Committee Review Standards)

- **Role here:** Procedural standard for consistent DAC review — informs **`DecisionRecord`** (who/when/what was evaluated).
- **Use in this project:** Every compatibility check yields a `decision_record_id` and stored audit row. Full DAC workflow (applications, votes) is **not** implemented.

## How the pieces fit

```text
Consent form (human)  ──MRCG──►  PolicyProfile (DUO codes)
Dataset policy        ──MRCG──►  PolicyProfile (DUO codes)
                                    │
                                    ▼
                         compatibility check (technical)
                                    │
                                    ▼
                         DecisionRecord (DACReS-oriented audit)
                                    │
                                    ▼
              Legal review + DAC discretion (outside this component)
```

## Further reading

- Dyke et al., consent codes underpinning DUO: https://doi.org/10.1371/journal.pgen.1005772
- EGA DUO adoption: https://ega-archive.org/access/data-access-committee/data-use-ontology/
- DUOS / dbGaP DUO alignment: https://pmc.ncbi.nlm.nih.gov/articles/PMC10504671/
