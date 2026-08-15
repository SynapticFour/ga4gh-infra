# Dev-only RSA signing keys — generated locally, never committed.

`make up` / `make prepare-secrets` writes three PKCS#8 PEMs here:

| File | Used by |
|------|---------|
| `broker_rs256.pem` | AAI broker Passport / access-token signing |
| `registry_rs256.pem` | Visa-registry visa JWTs |
| `mock_idp_rs256.pem` | Mock IdP (Compose only; omit in production) |

These files are gitignored. Keys that once lived in this directory are **public**
(git history). Do not reuse them. Treat any clone older than the removal commit
as having leaked those private keys.

## Generate

```bash
make prepare-secrets
# or:
ga4gh-infra keygen --output docker/secrets/broker_rs256.pem
openssl genpkey -algorithm RSA -pkeyopt rsa_keygen_bits:2048 \
  -out docker/secrets/broker_rs256.pem
chmod 600 docker/secrets/*.pem
```

`prepare-secrets` prefers `openssl`, then `ga4gh-infra keygen`, then `cargo run -p ga4gh-infra-cli`.

## Production

Do **not** mount this directory. Generate new keys onto a secret volume and
follow [key-rotation.md](../../docs/key-rotation.md). Production compose
examples point at `/run/secrets/…`, not `docker/secrets/`.
