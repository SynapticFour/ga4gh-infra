# ADS integration guide

How ADS integrates with GA4GH AAI, Passports, Visas, and resource services.

## GA4GH AAI broker

1. Researcher authenticates via broker (`aai-broker`).
2. Broker mints a Passport JWT (`sub` = researcher id).
3. For controlled-access datasets, the broker (or a visa-ingestion job) calls:
   - `GET /ads/v1/researchers/{sub}/visas` with the researcher's Bearer token
4. ADS returns unsigned `VisaClaim` objects (`ControlledAccessGrants`, `AffiliationAndRole`).
5. Broker or **visa-registry** signs claims into visa JWTs and embeds them in the Passport.

**Trust:** Configure ADS `oidc.trusted_brokers` with the broker issuer and JWKS URI (same as clearinghouse).

## Passports

Resource services receive the Passport as `Authorization: Bearer`. ADS does not replace
passport validation — it answers **authorization** questions via introspection:

```http
POST /ads/v1/introspect
X-API-Key: {resource-service-key}
Content-Type: application/json

{
  "token": "{passport-jwt}",
  "resource": "drs:ga4gh:doc:abc123",
  "dataset_id": "550e8400-e29b-41d4-a716-446655440000"
}
```

Response when permitted:

```json
{
  "active": true,
  "sub": "researcher@example.org",
  "grant_ids": ["..."],
  "duo_codes": ["GRU"],
  "exp": 1718190000
}
```

Combine with **ga4gh-clearinghouse** passport signature validation at the resource boundary.

## Visas

| Visa type | ADS source |
|-----------|------------|
| `ControlledAccessGrants` | Active grants (`resource_scope` or dataset id) |
| `AffiliationAndRole` | Researcher `affiliations` |

Revoking a grant (`DELETE /grants/{id}`) stops future introspection success and removes
the grant from visa export. Signed visas already in Passports remain until expiry — resources
should prefer introspection for fresh decisions.

## DRS

1. Register dataset with `external_id` = DRS object id prefix or bundle id.
2. On `GET /objects/{id}/access`, validate Passport signature (clearinghouse).
3. Call ADS introspect with DRS id as `resource`.
4. Allow access when `active: true`.

## Beacon v2

1. Register Beacon dataset id as `external_id`.
2. On `GET /beacon/beacon-xxx`, introspect with dataset id / beacon id.
3. Return `"exists": true` only when introspection is active (or gate full query endpoints).

## htsget

1. Map reference genome or file set to ADS dataset.
2. Before streaming bytes, introspect withhtsget reference id as `resource`.
3. Enforce DUO codes returned in introspection response if needed locally.

## WES / TES

1. Register workflow-accessible datasets in ADS.
2. At run submission, introspect Passport against workflow input dataset ids.
3. Reject runs when `active: false`.

## DUO service

ADS embeds DUO **compatibility evaluation** (`POST /duo/evaluate`, access request submission).
For authoritative DUO term definitions, deploy **duo-service** alongside ADS; ADS uses
`ga4gh-types` permission hierarchy aligned with duo-service matching direction.

## agreement-registry (optional)

For institution-specific agreement templates and richer compatibility (levels 2–3), integrate
**agreement-registry** as a future enhancement — ADS DAC workflow and `AccessDecision` records
are compatible with extended policy checks.

## Deployment checklist

- [ ] PostgreSQL (or SQLite for dev)
- [ ] `ADS_DATABASE_URL`, `ADS_DAC_API_KEY`
- [ ] Broker issuer in `oidc.trusted_brokers`
- [ ] Resource service API keys for introspection
- [ ] Dataset registration with DUO and `external_id`
- [ ] DAC operators use `X-API-Key` on `/dac/*` routes
