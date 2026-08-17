# SKETCH — not a production Kubernetes chart


Minimal Kubernetes chart for the identity plane. Terminate TLS at Ingress. Do not run `mock-idp`.

## Install

```bash
helm upgrade --install ga4gh deploy/k8s/chart \
  --namespace ga4gh --create-namespace \
  --set image.tag=REPLACE_WITH_RELEASE \
  --set broker.externalUrl=https://aai.example.org \
  --set adminUi.publicBaseUrl=https://admin.example.org
```

Create the `ga4gh-secrets` Secret before install (signing PEMs, API keys, cookie secret, `GA4GH_API_KEY_PEPPER`). See `values.yaml`.

Probes: `GET /health` (liveness), `GET /service-info` (readiness). Do not probe `/login`.
