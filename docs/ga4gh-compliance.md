# GA4GH profile checklist (self-assessment)

This is **not** a formal GA4GH certification. It records how this stack maps to published profiles so an institute DPO / architect can review gaps.

| Profile | What we implement | Gaps |
|---------|-------------------|------|
| [AAI OIDC Profile v1.2](https://ga4gh.github.io/data-security/aai-openid-connect-profile) | Broker is an RP; Passport JWT; `/userinfo`; JWKS; PKCE | No `/authorize` AS; `aud` not enforced (multi-audience Passports); no token refresh |
| [Passport & Visa v1.2](https://github.com/ga4gh-duri/ga4gh-duri.github.io/blob/master/researcher_ids/ga4gh_passport_v1.md) | Passport claims, embedded visa JWTs, visa types, conditions | Visa `jti` list plus broker Passport denylist (`/revoked-passports`); neither name is a GA4GH-standard endpoint |
| [Service Info](https://github.com/ga4gh-discovery/ga4gh-service-info) | `GET /service-info` on each service | — |
| [Service Registry](https://github.com/ga4gh-discovery/ga4gh-service-registry) | Read APIs + authenticated writes | No automatic self-registration from binaries (use `docker/scripts/register-service.sh`) |
| [DUO](https://github.com/EBISPOT/DUO) | `duo-service` OWL catalog + `ga4gh-types::evaluate_duo_codes` for ADS | ADS uses the published hierarchy, not the full OWL graph |

## Operator evidence to keep

- IdP client config (redirect `https://aai.example.org/callback`, PKCE)
- `allowed_return_url_origins` and `dac_operator_groups` values
- Signing-key ceremony and last rotation date
- Passport TTL and visa lifetime
- Proof that `mock-idp` is not deployed
- Checksum of the installed `ga4gh-infra` binary / image digest
