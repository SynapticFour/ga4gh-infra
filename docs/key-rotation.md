# Signing-key rotation

Broker and visa-registry mint RS256 JWTs from a PKCS#8 PEM. Resource services trust `iss` + JWKS `kid`. Rotation is a **planned overlap**, not a flag day.

## Inventory

| Key | Config | JWKS | JWT `iss` |
|-----|--------|------|-----------|
| Broker Passport / access token | `broker.signing.private_key_pem` | `GET {broker}/jwks.json` | `server.external_url` |
| Visa registry visas | `visa_registry.signing.private_key_pem` | `GET {visa-registry}/jwks.json` | visa-registry `external_url` |

Dev PEMs are generated under `docker/secrets/` and must never be copied into production. Keys that were previously committed in git are public — generate new ones.

## Zero-downtime rotation (overlap)

1. Generate a new key: `ga4gh-infra keygen --output /etc/ga4gh/secrets/broker_rs256.next.pem --bits 2048`.
2. Publish **both** keys in JWKS under distinct `kid` values:
   - **Preferred:** set `signing.previous_key_pems` on the new process to the old PEM (public or private). JWKS then contains both `kid`s; new tokens use the current key.
   - **Alternative:** start a second replica with the new key and the same `external_url` / `iss`, keep the old replica until minted JWTs expire.
3. Clearinghouses cache JWKS for `jwks_cache_ttl_seconds` (default 300). After switching, allow at least one TTL plus Passport TTL before deleting the old key file.
4. Record the rotation in the ops log (who, when, old `kid`, new `kid`). Admin-ui can show `signing_key_rotation_due` as a dashboard hint.

If you cannot run two replicas, schedule a maintenance window: Passport TTL is 900s in production examples, so a 20-minute window covers cache + expiry for broker keys. Visa JWTs may live longer (`visa_lifetime_seconds`); keep the old visa-registry key in JWKS until those visas expire or are revoked.

## Compromise

1. Revoke affected visas (`DELETE /visas/:id`) so `/revoked-jtis` lists them.
2. Rotate **both** broker and visa keys (an attacker with the broker key can mint Passports embedding still-valid visas).
3. Force re-login. Call broker `POST /revoke-passports` with `X-API-Key: $BROKER_ADMIN_API_KEY` and `{"sub":"..."}` (or `jti` / `visa_jti`) so clearinghouses reject live Passports via `GET /revoked-passports`.
4. Rotate API keys (`BROKER_COOKIE_SECRET`, `ADS_DAC_API_KEY`, `REGISTRY_BOOTSTRAP_API_KEY`, `GA4GH_API_KEY_PEPPER`, `BROKER_ADMIN_API_KEY`) if the host was breached.

## Multi-key JWKS

`signing.previous_key_pems` publishes extra RSA keys in `/jwks.json` without using them to sign. Use it for overlap in a single replica. Remove the previous PEM after TTL + JWKS cache.
