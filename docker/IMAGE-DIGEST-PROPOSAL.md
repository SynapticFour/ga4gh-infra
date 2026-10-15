# Compose image digest proposal

Compose files were not edited. Index digests below were resolved with
`docker buildx imagetools inspect` on 2026-10-14. A tag in the image
reference is the tag that was inspected, not a new pin in Compose.

Default component tags are the values in `docker/.env.example` (`0.2.3`).
`GA4GH_IMAGE_PREFIX` there is `ghcr.io/synapticfour`.

| Compose image | Tag inspected | Index digest |
|---|---|---|
| `postgres:16-alpine` | `postgres:16-alpine` | `sha256:721873c34ceb9f8d8fc265984940dc982404c105f19ad51be9fdc5970a6080ea` |
| `${GA4GH_IMAGE_PREFIX}/mock-idp:${MOCK_IDP_VERSION}` | `ghcr.io/synapticfour/mock-idp:0.2.3` | `sha256:8e6339606329ccb229878cc8face327cd9d2153d9c402f25f0bc2e7c892fe331` |
| `${GA4GH_IMAGE_PREFIX}/visa-registry:${VISA_REGISTRY_VERSION}` | `ghcr.io/synapticfour/visa-registry:0.2.3` | `sha256:5233764a5480f239323e73a2b004c7aaa183214d650396acef08b12515999d98` |
| `${GA4GH_IMAGE_PREFIX}/duo-service:${DUO_SERVICE_VERSION}` | `ghcr.io/synapticfour/duo-service:0.2.3` | `sha256:0b487172d4e4e23f996265ab1d6c3441e9a3d5d58024e1c55d61c82edde80894` |
| `${GA4GH_IMAGE_PREFIX}/service-registry:${SERVICE_REGISTRY_VERSION}` | `ghcr.io/synapticfour/service-registry:0.2.3` | `sha256:a12dbc58e3816b880679ddaff32c999e50c4ec8f09f8b63ce8e9dfd07710ec4c` |
| `${GA4GH_IMAGE_PREFIX}/access-decision-service:${ACCESS_DECISION_SERVICE_VERSION}` | `ghcr.io/synapticfour/access-decision-service:0.2.3` | `sha256:afb6794b70afe43dd68047b9c9337cfc5670937909fbeff16d1aa4d3ba433b08` |
| `${GA4GH_IMAGE_PREFIX}/aai-broker:${AAI_BROKER_VERSION}` | `ghcr.io/synapticfour/aai-broker:0.2.3` | `sha256:bed63ac0f6f1d2343ac84c45ba41612bd93c2c12adfcb51ea18a38b77db7b474` |
| `${GA4GH_IMAGE_PREFIX}/sample-resource:${SAMPLE_RESOURCE_VERSION}` | `ghcr.io/synapticfour/sample-resource:0.2.3` | `sha256:73fbef8727812866ad927b1b242b5e91e293ca0a1aab8e2bb4877012d2603b52` |
| `${GA4GH_IMAGE_PREFIX}/agreement-registry:${AGREEMENT_REGISTRY_VERSION:-0.2.3}` | `ghcr.io/synapticfour/agreement-registry:0.2.3` | `sha256:562d84cdd17c2ce9c203f276e51bfb296e6e800bee8602a10676de8e6204e56a` |
| `${GA4GH_IMAGE_PREFIX}/admin-ui:${ADMIN_UI_VERSION:-0.2.3}` | `ghcr.io/synapticfour/admin-ui:0.2.3` | `sha256:3b8d393f1d2fe0df8af172d03c32e680f64253b62f738911bf7e9792f83a3661` |
| `${GA4GH_IMAGE_PREFIX}/register-tools:${SERVICE_REGISTRY_VERSION}` | `ghcr.io/synapticfour/register-tools:0.2.3` | `sha256:0f78339e99e1fa8a42545ba02ee26609ea698ebc6464a6d5d724b2f18ad0465f` |

Files that contain these references and were not edited:

- `docker/docker-compose.yml`
- `docker/docker-compose.sqlite.yml` (no Postgres service; the GHCR rows above still apply)
- `docker/docker-compose.prod.example.yml` (`postgres:16-alpine` and a subset of the GHCR rows)

`.github/workflows/docker-release.yml` still publishes a `:latest` tag next to the
version tag. That file is not a Dockerfile, so the tag was left in place.
