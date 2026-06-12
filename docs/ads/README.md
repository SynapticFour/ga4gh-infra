# Access Decision Service (ADS)

GA4GH Access Decision Service documentation for the `access-decision-service` crate.

| Document | Description |
|----------|-------------|
| [architecture.md](architecture.md) | Components, auth model, deployment |
| [domain-model.md](domain-model.md) | Researchers, datasets, grants, access requests |
| [database-schema.md](database-schema.md) | PostgreSQL/SQLite schema |
| [events.md](events.md) | Audit events and webhooks |
| [integration.md](integration.md) | Broker, visa-registry, and resource service integration |
| [sequence-diagrams.md](sequence-diagrams.md) | Login, DAC approval, and introspection flows |
| [openapi.yaml](openapi.yaml) | REST API reference |

Run standalone:

```bash
ga4gh-infra access-decision-service --config config/ads.toml
```

Docker stack exposes ADS on port **8090** when using `docker/docker-compose.yml`.
