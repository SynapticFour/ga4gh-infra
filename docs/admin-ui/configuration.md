# Admin UI configuration

Configuration is a single TOML file passed via `--config` or `ADMIN_UI_CONFIG`.

## Example

```toml
listen_addr = "0.0.0.0:8095"
public_base_url = "http://localhost:8095"
broker_base_url = "http://localhost:8080"
ads_base_url = "http://localhost:8090"
ads_dac_api_key = "dev-ads-api-key"
duo_base_url = "http://localhost:8082"
visa_registry_base_url = "http://localhost:8081"
service_registry_base_url = "http://localhost:8083"
session_secret = "change-me-to-at-least-32-random-characters"
session_ttl_hours = 24
admin_claim = "groups"
admin_claim_value = "ga4gh-infra-admins"
```

Copy from `config/admin-ui.example.toml` for local development.

## Fields

| Field | Required | Description |
|-------|----------|-------------|
| `listen_addr` | yes | Socket address (e.g. `0.0.0.0:8095`) |
| `public_base_url` | yes | External URL of admin-ui (used in broker return URL and callback fetch) |
| `broker_base_url` | yes | AAI broker base URL for login redirect |
| `ads_base_url` | yes | Access Decision Service base URL |
| `ads_dac_api_key` | yes | DAC API key for server-side ADS calls |
| `duo_base_url` | yes | DUO service base URL (dataset form term picker) |
| `visa_registry_base_url` | yes | Visa registry base URL (health probe) |
| `service_registry_base_url` | yes | Service registry base URL (health probe) |
| `session_secret` | yes | HMAC secret for session cookie (min 32 chars) |
| `session_ttl_hours` | no | Session lifetime (default `24`) |
| `admin_claim` | no | OIDC claim for group list (default `groups`) |
| `admin_claim_value` | no | Group value for Admin role (default `ga4gh-infra-admins`) |
| `static_dir` | no | Override path to static assets (default: crate `static/`) |

## Environment

When using the standalone binary:

```bash
export ADMIN_UI_CONFIG=/path/to/admin-ui.toml
admin-ui
```

Via combined CLI:

```bash
ga4gh-infra admin-ui --config config/admin-ui.example.toml
```

## Docker

Docker Compose mounts `docker/config/admin-ui.toml` at `/config/admin-ui.toml`. Override secrets (especially `session_secret` and `ads_dac_api_key`) for non-dev deployments.

## Security notes

- Use HTTPS in production; set `public_base_url` to the TLS URL.
- Rotate `session_secret` and `ads_dac_api_key` independently.
- The callback flow reads the access token from the URL **fragment** (not logged server-side by the broker redirect).
