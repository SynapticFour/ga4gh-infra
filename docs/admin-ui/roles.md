# Admin UI roles

Admin-ui distinguishes **Operator** and **Admin** using OIDC group membership from the broker access token.

## Operator

Authenticated users who can:

- View the dashboard and service health
- View and refresh the DAC queue
- Approve, reject, or escalate access requests
- Browse registered datasets and open detail pages

## Admin

Users whose OIDC token includes the configured admin group value (default **`ga4gh-infra-admins`** in the `groups` claim) gain Operator access plus:

- Register new datasets via the form on `/datasets`

## Configuration

```toml
admin_claim = "groups"
admin_claim_value = "ga4gh-infra-admins"
```

| Key | Default | Meaning |
|-----|---------|---------|
| `admin_claim` | `groups` | JWT claim holding group strings (array or single string) |
| `admin_claim_value` | `ga4gh-infra-admins` | Value that grants Admin role |

## Mock IdP / development

The bundled mock IdP does not emit `groups` by default, so development logins are **Operator** unless you extend mock token claims or test with a real IdP.

To exercise Admin locally, decode a test JWT with `"groups": ["ga4gh-infra-admins"]` and POST it to `/auth/session`, or configure your IdP to issue the admin group.

## ADS authorization

DAC API calls from admin-ui always use the service **`ads_dac_api_key`**. End-user JWT roles do not map to ADS API keys in Phase 9; UI roles only gate browser-facing actions (e.g. dataset creation form).
