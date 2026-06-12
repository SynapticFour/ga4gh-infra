# Reverse proxy examples

TLS termination configs for production deployments. Services in the Compose stack listen on plain HTTP; expose them to the internet only through one of these proxies.

| File | Use when |
|------|----------|
| [`Caddyfile.example`](Caddyfile.example) | You want automatic Let's Encrypt and minimal config |
| [`nginx.conf.example`](nginx.conf.example) | You already run nginx with manual or certbot certificates |

**Before go-live:**

1. Set each service's `external_url` to the matching `https://` hostname (see [docs/production-deployment.md](../../docs/production-deployment.md)).
2. Point `proxy_pass` / `reverse_proxy` at Compose ports (host) or Docker network service names (if the proxy runs inside Compose).
3. Do not expose Postgres or the visa-registry DAC API to the public internet without additional access controls.

See the full guide: [docs/production-deployment.md](../../docs/production-deployment.md).
