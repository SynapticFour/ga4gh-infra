# Passport size (researcher showcase shape)

Evidence for ADR-005. Helix baseline: HelixTest already runs; Helix productizes conformance/security/benchmark; evidence stays HELIOS. This note is ga4gh-infra broker sizing, not Helix product work.

Measured 2026-10-12 with the same RS256-2048 PEMs shipped under `docker/secrets/` (dev keys), Visa claim shapes matching s4-showcase `seed/seed_demo.py`, and 22 Visa JWTs.

## Why there are 22 Visas

Showcase `desired_visas()` for `researcher`:

| Count | Type | Why |
|------:|------|-----|
| 1 | `ResearcherStatus` = `researcher` | Bona-fide researcher attribute |
| 21 | `ControlledAccessGrants` | One grant per seeded DRS alias |

Datasets = 4 synthetic aliases (`synthetic-cohort`, `synthetic-variants`, `synthetic-regions`, `giab-reference-catalog`) + **17** files from `deploy/state/reference/manifest.json` → **21** grants. Total **22**.

Other personas get one `AffiliationAndRole` each (small tokens).

## Byte breakdown (embedded mode, default)

| Part | Bytes (approx) | Notes |
|------|---------------:|-------|
| Visa JWT header (b64) | 59 each | `{"alg":"RS256","typ":"JWT","kid":…}` |
| Visa JWT claims (b64) | ~510–527 each | `sub`, `iss`, `iat`, `exp`, `jti`, `ga4gh_visa_v1`, `jku` |
| Visa JWT signature (b64) | **342** each | RS256, **2048-bit** registry key |
| One Visa JWT | ~900–930 | |
| **Sum of 22 Visa JWTs** | **~20 137** | Dominates the Passport |
| Outer Passport header | 56 | Broker RS256-2048 |
| Outer Passport claims | ~27 378 | Includes JSON-encoded Visa strings |
| Outer Passport signature | 342 | |
| **Passport JWT** | **~27 778** | Showcase laptop ~28 475 with live `sub`/URLs/`groups` |
| `Authorization: Bearer …` | ~27 785+ | Forces Caddy `:443` `MaxHeaderBytes` **64 KiB** |

Signature share: 22 × 342 ≈ **7.5 KiB** of Visa signatures alone.

Regression test: `aai-broker` `showcase_scale_embed_vs_userinfo_access_token_size` (22 ~900 B stubs; embed on > 16 KiB Authorization; embed off < 2 KiB).

## Options (sizes vs GA4GH Passport / AAI)

| Option | Measured / estimated size | Spec fit | Trade-off |
|--------|---------------------------|----------|-----------|
| **A. Shorter claims / values** | Omit `jku` ≈ tens of bytes × 22; shorter `source`/`value` similar | Passport Visa claim fields are largely fixed; `jku` helps key discovery | Small win; does not remove Caddy pressure |
| **B. Fewer / merged Visas** | 1+N grants; N=1 → ~2 KiB Passport | Passport v1: one `value` per `ControlledAccessGrants` Visa — no merge of many datasets into one Visa | Showcase can grant fewer aliases; not a general broker fix |
| **C. Smaller alg / key** | ES256 sig ≈ ~86 B b64 vs 342 → save ~5.6 KiB → still ~22 KiB Passport | AAI allows listed algs; Clearinghouses must accept ES256 | Breaking for RS256-only verifiers; still large |
| **D. Small access token + UserInfo Visas** | Access token **~842 B** (empty `ga4gh_passport_v1`); Visas on `GET /userinfo` | **GA4GH AAI**: Passport-scoped access token + UserInfo MUST return `ga4gh_passport_v1`; Token Exchange preferred for a self-contained Passport | **Recommended durable fix.** Consumers must fetch/cache UserInfo. ADR-005 flag `passport.embed_visas` (default `true`) |

## Recommendation

1. Keep **`passport.embed_visas = true`** (default) until Ferrum, Solum, and in-repo Clearinghouses that need Visas are on a release with UserInfo fetch.
2. Prefer **D** for public edges: set `embed_visas = false`, then lower Caddy header limits.
3. Do not switch Visa signing to ES256 by default in this release.
4. Showcase may still reduce grant count for demos, but that is orthogonal.

## Consumer matrix (`embed_visas = false`)

| Consumer (this repo) | Status with `embed_visas = false` | Notes |
|----------------------|-----------------------------------|--------|
| **admin-ui** | **Supported** | Session is built from outer Passport claims (`sub`, `name`, `email`, admin `groups` claim). It does **not** read `ga4gh_passport_v1`. Works as long as those identity claims remain on the access token (unchanged by ADR-005). |
| **access-decision-service (ADS)** | **N/A** | Issues Visas / permissions; not a Passport resource server (Bearer used for researcher identity / introspect, not Visa policy). |
| **duo-service** | **N/A** | Ontology / codes; not a Passport resource server. |
| **ga4gh-clearinghouse** | **Supported** (proposed `ga4gh-infra-v0.2.7`) | Empty `ga4gh_passport_v1` → OIDC discovery of trusted `iss` → `GET /userinfo` (ADR-006; same rules as Ferrum ADR-034). |
| **sample-resource** | **Supported** (via clearinghouse) | Passes Bearer into `extract_visas`. |
| **visa-registry / agreement-registry / service-registry / mock-idp** | **N/A** | Not Passport Clearinghouses for broker access tokens. |

Out-of-repo: **Ferrum** (external auth) fetches UserInfo when `ga4gh_passport_v1` is empty (ADR-034; Ferrum `v0.3.5`). **Solum** same pattern when it trusts broker Passports.

Do not put the Visa document store behind the public apex without auth. UserInfo already requires a valid Bearer.

## In-memory store bounds and restart

| Bound | Value | Behaviour |
|------:|-------|-----------|
| TTL | Passport `exp` | Expired documents are purged on insert and ignored on get |
| Max entries | 10 000 live documents | Insert refused → mint fails closed (500) |
| Max Visa bytes | 64 MiB total JWT string bytes | Insert refused → mint fails closed |
| Restart | Store empty | Valid token + empty claim + missing document → `/userinfo` **401** (fail closed); re-login |

## Multi-replica

The Visa document cache is **in-memory per broker process**. HA with `embed_visas = false` needs a shared store (follow-up) or sticky sessions to the issuing replica. A restart of any replica that minted the token fails closed for that token’s UserInfo until the researcher logs in again on a live replica that still holds the document.
