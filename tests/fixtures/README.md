# Shared test fixtures

Workspace-wide test assets referenced by unit and integration tests.

| Path | Purpose |
|------|---------|
| `sample_visa_payload.json` | Minimal unsigned visa assertion JSON for handler tests |
| `sample_match_request.json` | DUO `/match` request body |
| `sample_service_registration.json` | GA4GH service registration payload |

RSA keys and JWTs are generated deterministically in test helpers (`ga4gh-clearinghouse/tests/support.rs`, crate `test_support` modules) to avoid committing private key material beyond dev `docker/secrets/`.

Integration tests load these files via `CARGO_MANIFEST_DIR/../fixtures/`.
