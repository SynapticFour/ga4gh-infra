# ga4gh-infra — Threat model addendum

**Status:** Living · 2026-08-12
**Audience:** Operators co-deploying with Ferrum
**Related:** [limitations.md](limitations.md) · [architecture.md](architecture.md) · Ferrum [THREAT_MODEL.md](https://github.com/SynapticFour/Ferrum/blob/main/docs/THREAT_MODEL.md)

This is a **short identity-plane addendum**, not a full STRIDE document. When Ferrum uses `[auth] mode = "external"`, **ga4gh-infra becomes part of Ferrum’s trust boundary**.

---

## Assets

| Asset | Risk if compromised |
|-------|---------------------|
| OIDC broker / IdP integration | Issue tokens for wrong subjects |
| Passport / visa signing keys | Forge visas → Ferrum grants data/compute |
| DUO / ADS policy config | Over-broad access decisions |
| Service registry entries | Misdirect clients to malicious endpoints |
| Admin UI credentials | Full identity-plane control |

---

## Adversaries (in scope)

- External attacker against broker/visa HTTP surfaces
- Stolen admin session
- Misconfigured mock-IdP left enabled in production
- Supply-chain compromise of infra binaries

## Out of scope / honesty

See [limitations.md](limitations.md). Passport/visa flows may involve identifiable researcher data — legal basis is operator-owned. Offline / edge visa constraints must be documented to pilots (do not imply full online AAI on a disconnected Pi).

---

## Operator checklist (co-deploy)

1. Disable mock-IdP in production.
2. Protect visa signing material; rotate on staff change.
3. TLS on all public ports (8180–8190 class).
4. Align Ferrum JWKS/issuer with infra.
5. Include infra hosts in the same IR / backup plan as Ferrum.

---

## Residual risks

Identity-plane bugs or misconfiguration are **high impact** (they unlock Ferrum). Prefer HelixTest `ferrum+infra` profiles before production cutover; optional HelixTest auth-on live remains **backlog-only** under the spine freeze unless a pilot requires it.
