# Agreement registry — limitations

## What this component does

- Stores **PolicyProfile** and **AgreementTemplate** data structures (library today; HTTP service planned).
- Performs **technical DUO compatibility** between requester and dataset code sets.
- Emits **DecisionRecord** audit rows suitable for DACReS-style reporting.

## What it does not do

| Gap | Why |
|-----|-----|
| Legal review | Consent prose, MTAs, GDPR/HIPAA, and national law require counsel — not automatable from DUO alone |
| Replace DAC discretion | `compatible: true` means DUO codes align, not that access must be granted |
| Parse consent PDFs | No NLP over consent forms; operators map text using MRCG manually |
| Resolve `source_document_ref` | Opaque pointer to Ferrum or other DTA stores — not fetched or validated here |
| Full DAC workflow | No applications, voting, or e-signatures (level 4 extension point) |
| Guarantee template accuracy | Seed templates document real-world **patterns**; only one template is explicitly illustrative |

## The `conditions` field exists on purpose

`CompatibilityCheckResult.conditions` lists items DUO cannot fully automate, for example:

- Ethics approval (`IRB`) must be confirmed by a DAC member
- Disease-specific (`DS`) dataset missing `modifier_value` (MONDO id)
- Template requires visa types not present on the requester profile
- Dataset references external legal text via `source_document_ref`
- Matched template is marked `is_illustrative: true`

**A result may be `compatible: true` with non-empty `conditions`.** Operators must not treat an empty `unsatisfied_codes` list as legal approval.

## DUO vs legal agreement

DUO codes standardize **technical** data-use restrictions. They do not capture every nuance of consent (e.g. return of results, commercial spin-out clauses, cross-border transfer mechanics). Document unmappable clauses in institutional policy text and handle them in level 1 / level 4 systems.

## Security review

Like the rest of `ga4gh-infra`, this component has **not** undergone formal third-party security audit. Do not expose decision records containing identifiable researcher metadata without appropriate access controls.
