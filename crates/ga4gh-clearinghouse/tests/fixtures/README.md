# GA4GH Clearinghouse test fixtures

These files contain **test-only** RSA keys and JWTs used by `ga4gh-clearinghouse`
integration tests. They must never be used in production.

The tokens are shaped like ELIXIR Life Science AAI / GA4GH Passport examples:

- Passport issuer: `https://test-broker.example.org`
- Visa issuer: `https://test-visas.example.org`
- Standard visa types: `AffiliationAndRole`, `ResearcherStatus`, `ControlledAccessGrants`
- Passport claim: `ga4gh_passport_v1` array of compact-serialization visa JWT strings

JWTs are generated deterministically in `tests/support.rs` using a fixed RNG seed so
tests remain reproducible. The JWKS served in tests is mounted via Wiremock from the
same key material.
