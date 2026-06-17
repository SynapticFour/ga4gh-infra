# Admin UI roles

Admin-ui distinguishes **Operator** and **Admin** using OIDC group membership from the broker access token. There is a single login — Admin is a higher-privilege view of the same UI, not a separate account.

## Operator (default)

Authenticated users who can perform day-to-day operations:

| Page | Access | Rationale |
|------|--------|-----------|
| Dashboard | View | Service health and activity summary; scoped counts |
| DAC Queue | View + actions | Approve/reject/escalate requests for the user's DAC group(s) |
| Datasets | View + search | Browse registered datasets in scope |
| Projects | View + search | Browse research projects |
| Grants | Scoped view | Own grants plus grants for datasets in the user's DAC group(s) |
| Audit Log | Scoped view | Events for the user's DAC group(s) and own actions |
| Service Registry | View | See registered GA4GH services and health |

Operators **cannot** register datasets, revoke arbitrary grants, manage service registry entries, view all researchers, edit agreement templates, or change system configuration.

## Admin

Users whose OIDC token includes the configured admin group value (default **`ga4gh-infra-admins`** in the `groups` claim) gain Operator access plus:

| Page | Additional access | Rationale |
|------|-------------------|-----------|
| Dashboard | Signing key summary + rotation warning | Operational trust-root visibility |
| Datasets | Create + edit forms | Dataset registration is deployment configuration |
| Projects | Create + edit forms | Project registration on behalf of researchers |
| Grants | All grants + revoke + CSV export | Cross-institution compliance view |
| Audit Log | Full event stream (no DAC filter) | Compliance and incident review |
| Service Registry | Register/update/delete entries | Manual registration for non-self-registering services |
| Researchers | Search + visa preview | Cross-researcher support (Admin only) |
| Agreements | Templates, profiles, compatibility check | Institutional partnership evaluation |
| System | IdP read-only view, JWKS, permission mappings CRUD | Safe live config vs trust-root config |

Admin-only sections appear under a distinct **Admin** nav heading so operators are not confused by actions they cannot use.

## Configuration

```toml
admin_claim = "groups"
admin_claim_value = "ga4gh-infra-admins"
```

| Key | Default | Meaning |
|-----|---------|---------|
| `admin_claim` | `groups` | JWT claim holding group strings (array or single string) |
| `admin_claim_value` | `ga4gh-infra-admins` | Value that grants Admin role |

## ADS authorization

DAC API calls from admin-ui use the service **`ads_dac_api_key`**. End-user JWT roles gate browser-facing pages only; ADS enforces its own API key and researcher scoping on grant endpoints.

## Phase 10+ ideas

The following are intentionally **read-only or file-based in Phase 9**:

- **Live upstream IdP editing** — IdP issuer, client_id, and claim mappings require editing the broker config file (`broker_config_path`) and restarting the broker. A future phase could add validated live IdP CRUD with audit trails and staged rollout.
- **Signing key rotation via UI** — Phase 9 shows JWKS fingerprints and optional `signing_key_rotation_due` warnings; actual key rotation remains an operator runbook (generate key, update PEM, rolling JWKS publish, restart).
- **Agreement registry persistence** — HTTP service is in-memory with seed templates; production would add durable profile/template storage.

## Mock IdP / development

The bundled mock IdP does not emit `groups` by default, so development logins are **Operator** unless you extend mock token claims.

To exercise Admin locally, configure your IdP to issue `"groups": ["ga4gh-infra-admins"]`, or POST a test JWT with that claim to `/auth/session`.
