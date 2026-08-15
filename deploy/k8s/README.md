# Kubernetes notes

Starter Helm chart: [chart/](chart/). Probe conventions below still apply if you write manifests by hand.

## Probes

| Endpoint | Use |
|----------|-----|
| `GET /health` | liveness (process up) |
| `GET /service-info` | readiness (config loaded) |

Do not use `/login` as a probe (it starts an OIDC flow and is rate-limited).

## Trust and URLs

- Pod-to-pod HTTP is fine (`http://aai-broker:8080/jwks.json`).
- JWT `iss` and `server.external_url` must be the **public HTTPS** URL.
- Clearinghouse `trusted_issuers[].issuer` matches `iss`; `jwks_uri` may be the in-cluster URL.

## What not to run

- `mock-idp`
- Laptop-generated `docker/secrets/*.pem` (production: `ga4gh-infra keygen` onto a Kubernetes Secret)
- A single replica of the broker if you need signing-key overlap ([key-rotation.md](../docs/key-rotation.md))

## Example broker Deployment (sketch)

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: aai-broker
spec:
  replicas: 2
  selector:
    matchLabels: { app: aai-broker }
  template:
    metadata:
      labels: { app: aai-broker }
    spec:
      containers:
        - name: aai-broker
          image: ghcr.io/synapticfour/aai-broker:REPLACE_WITH_RELEASE
          args: ["/config/broker.toml"]
          ports:
            - containerPort: 8080
          envFrom:
            - secretRef: { name: ga4gh-broker }
          volumeMounts:
            - name: config
              mountPath: /config
              readOnly: true
            - name: secrets
              mountPath: /secrets
              readOnly: true
          livenessProbe:
            httpGet: { path: /health, port: 8080 }
          readinessProbe:
            httpGet: { path: /service-info, port: 8080 }
      volumes:
        - name: config
          configMap: { name: aai-broker }
        - name: secrets
          secret: { secretName: ga4gh-signing-keys }
```

Terminate TLS at Ingress. Rate-limit `/login` and `/callback` at the Ingress or a mesh policy in addition to the in-process limiter.
