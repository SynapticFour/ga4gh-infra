# Governance

`ga4gh-infra` is an identity and access plane. A research organization must be able to **patch it without waiting on a single volunteer**.

## Development vs production

This upstream repository is **still in development**. `main` is **not** branch-protected: the maintainer pushes directly, and CI runs on those pushes. `.github/CODEOWNERS` names an owner for later dual-control; it does **not** require pull-request reviews and must not be wired to required reviews until a second human is actually reviewing.

Institutes going to production should fork, then apply the checklist below on **their** copy. Do not enable required reviews on this upstream `main` while it is a one-person project.

## Maintainers

GitHub CODEOWNERS currently lists `@SynapticFour`. That is **bus factor 1**. Before **your** production go-live:

1. Fork or add your institute GitHub team to [`.github/CODEOWNERS`](../.github/CODEOWNERS) (identity, ADS, visa-registry, installers, `.github/`).
2. On that fork, enable **branch protection** on `main`: require a pull request, require CODEOWNER review, dismiss stale reviews, do not allow bypass for administrators except break-glass.
3. Name a **deputy** who can cut a signed `ga4gh-infra-v*` release (checksums + GHCR images) if the primary maintainer is unavailable.
4. Keep a local clone and the signing-key ceremony documented in [key-rotation.md](key-rotation.md) so you are not blocked on GitHub.

Upstream contributions are welcome; production operators should not treat `SynapticFour/ga4gh-infra` as the only copy of the source.

## Dual control

These changes always need a second reviewer (even if that reviewer is at your institute, not upstream):

- Broker login / `return_url` / Passport minting
- DAC authorization (admin-ui roles, ADS `operator_groups`)
- Installers and release checksums
- Signing-key or JWKS handling

## Releases

Do not run `scripts/install.sh` against `ga4gh-infra-v0.1.0`. Current stack tag:

```bash
git tag ga4gh-infra-v0.2.2
git push origin ga4gh-infra-v0.2.2
```

That tag publishes checksummed binaries **and** (after the stack Docker Release) every Compose image as `:0.2.2`. Confirm both `Release Binaries` and `Docker Release` succeeded before pointing production pins at the version.

## Incident response

Private reports: [SECURITY.md](../SECURITY.md). Rotate Passport and visa signing keys after a suspected leak ([key-rotation.md](key-rotation.md)). Revoke visas (`/revoked-jtis`) and Passports (`POST /revoke-passports`) and keep Passport TTL short.
