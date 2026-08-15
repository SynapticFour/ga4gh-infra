# Institute identity provider wiring

The broker is an OIDC **Relying Party**. Researchers authenticate at your IdP; the broker mints GA4GH Passports. SAML-only IdPs need an OIDC front (Keycloak, Shibboleth OIDC plugin, Entra).

Production `external_url` must be the public HTTPS URL. Register `{external_url}/callback` as the redirect URI.

## Groups (required for admin-ui / DAC)

Admin-ui and DAC scoping use the Passport `groups` claim (configurable `admin_claim`). Map IdP groups into that claim.

| Role | Typical group |
|------|----------------|
| Platform admin | `ga4gh-infra-admins` (`admin_claim_value`) |
| DAC operator | values listed in admin-ui `dac_operator_groups` (must match ADS request `dac_group`) |

## Keycloak

Full broker TOML: [`config/broker.keycloak.example.toml`](../config/broker.keycloak.example.toml).

1. Create a confidential client `ga4gh-broker`, Standard flow on, Direct access grants off.
2. Valid redirect URI: `https://aai.example.org/callback`.
3. Client scopes: `openid`, `profile`, `email`.
4. **Groups mapper:** Client → Client scopes → dedicated mappers → *Group Membership* (or protocol mapper Token claim name `groups`, full path off). Without this, nobody is an admin or DAC operator.
5. Create groups `ga4gh-infra-admins` and your DAC group names; assign users.
6. Client secret → `MY_INSTITUTE_CLIENT_SECRET`.
7. Issuer must match realm URL exactly: `https://idp.example.org/realms/your-realm`.

Optional: mapper for `eduperson_scoped_affiliation` if you populate `claim_mapping.affiliation`.

## Microsoft Entra ID

1. App registration, platform **Web**, redirect `https://aai.example.org/callback`.
2. Certificates & secrets → client secret → `MY_INSTITUTE_CLIENT_SECRET`.
3. Token configuration → ID token optional claims (`email`, `preferred_username`).
4. Groups: Token configuration → add `groups` claim (security groups). 200+ groups may emit an overage claim — keep DAC groups few, or map a dedicated app role.
5. Issuer: `https://login.microsoftonline.com/{tenant-id}/v2.0`.
6. Broker `claim_mapping.groups = "groups"` (default).

## ELIXIR / LS Login (OIDC)

1. Register a confidential client with the LS AAI operator; redirect `https://aai.example.org/callback`.
2. Request scopes `openid`, `profile`, `email`, plus any entitlement scope they document.
3. Issuer is the LS Login discovery issuer (copy from `.well-known/openid-configuration`).
4. Map entitlements: if they arrive as `eduperson_entitlement` rather than `groups`, set `claim_mapping.groups = "eduperson_entitlement"`.

## Shibboleth (OIDC plugin)

Use the IdP’s OIDC OP plugin (not SAML ACS). Redirect URI and issuer come from that OP metadata. Map `memberOf` / `isMemberOf` onto `groups` via `claim_mapping`.

## Checklist

- [ ] `/login/{idp_name}` redirects to the IdP
- [ ] Callback returns a Passport; `/userinfo` shows `sub`
- [ ] `groups` in the Passport includes the admin and DAC values you configured
- [ ] `allowed_return_url_origins` includes only admin-ui (and other first-party UIs)
- [ ] `mock-idp` is not running
