# Security policy

## Supported versions

| Version | Supported |
|---------|-----------|
| Latest `0.x` release | Yes |
| Older minors / tags | Best effort only |

Run the latest patch of the current release. Pin Docker image tags in production (`docker/.env`).

---

## Reporting vulnerabilities

1. **Do not** open a public GitHub issue for a security vulnerability.
2. Email [contact@synapticfour.com](mailto:contact@synapticfour.com) or use **GitHub Security Advisories** on this repository (**Security** → **Report a vulnerability**).
3. Include description, reproduction steps, and impact. We will acknowledge receipt and work on a fix and coordinated disclosure.
4. After a fix is released, we may publish an advisory and credit you unless you prefer to remain anonymous.

---

## Security model (summary)

| Area | Behaviour |
|------|-----------|
| **Broker** | OIDC Relying Party (not an Authorization Server). Upstream login via institute IdP; short-lived signed session cookies for the RP flow. |
| **Passports / visas** | RS256 JWTs; resource services validate via `ga4gh-clearinghouse` (signature, issuer trust, expiry). |
| **Visa registry DAC API** | API key auth; should not be exposed on the public internet without network controls. |
| **Service registry writes** | Shared registration key; use `read_only = true` on public deployments. |
| **Dev keys** | PEM files under `docker/secrets/` are **test-only** — never use in production. |

This project has **not** undergone a formal third-party security audit. See [docs/limitations.md](docs/limitations.md).

---

## Hardening checklist for operators

- Terminate TLS at a reverse proxy; set correct `external_url` values.
- Replace all dev secrets and signing keys before production.
- Run Postgres and internal APIs on private networks.
- Keep image and crate versions pinned and updated.
- Deploy behind rate limiting / WAF where appropriate (not built into this stack).
