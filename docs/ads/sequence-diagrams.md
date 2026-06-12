# ADS sequence diagrams

## Access request with DUO auto-approval

```mermaid
sequenceDiagram
  participant R as Researcher
  participant ADS as ADS
  participant DB as Database

  R->>ADS: POST /projects (Bearer JWT)
  ADS->>DB: insert research_project
  R->>ADS: POST /access-requests
  ADS->>DB: load dataset + project DUO
  ADS->>ADS: evaluate DUO compatibility
  alt auto_approvable and dataset.auto_approve_enabled
    ADS->>DB: insert request (approved)
    ADS->>DB: insert grant + audit events
  else requires DAC
    ADS->>DB: insert request (pending)
  end
  ADS-->>R: AccessRequest
```

## DAC review workflow

```mermaid
sequenceDiagram
  participant DAC as DAC reviewer
  participant ADS as ADS
  participant DB as Database

  DAC->>ADS: GET /dac/requests (X-API-Key)
  ADS->>DB: list pending/escalated
  ADS-->>DAC: queue
  DAC->>ADS: POST /dac/requests/{id}/approve
  ADS->>DB: insert AccessDecision
  ADS->>DB: update request status
  ADS->>DB: insert Grant
  ADS->>DB: audit grant.created, request.approved
  ADS-->>DAC: approved AccessRequest
```

## Passport introspection (DRS / Beacon / htsget / WES / TES)

```mermaid
sequenceDiagram
  participant RS as Resource service
  participant ADS as ADS
  participant JWKS as Broker JWKS

  RS->>ADS: POST /introspect (X-API-Key + Passport token)
  ADS->>JWKS: verify Passport JWT
  ADS->>ADS: lookup active grants for sub + resource
  alt grant found and not expired/revoked
    ADS-->>RS: { active: true, grant_ids, duo_codes }
    RS->>RS: serve resource
  else no grant
    ADS-->>RS: { active: false, reason }
    RS->>RS: 403 Forbidden
  end
```

## Visa export for AAI passport assembly

```mermaid
sequenceDiagram
  participant Broker as AAI broker
  participant ADS as ADS
  participant VR as Visa registry

  Broker->>ADS: GET /researchers/{sub}/visas (Bearer)
  ADS->>ADS: grants + affiliations → VisaClaim[]
  ADS-->>Broker: ResearcherVisasResponse
  Broker->>VR: POST /visas (sign claims)
  VR-->>Broker: visa JWTs
  Broker->>Broker: embed in Passport ga4gh_passport_v1
```

## Institutional permission mapping (configuration)

```mermaid
sequenceDiagram
  participant Admin as ADS admin
  participant ADS as ADS
  participant IdP as Institutional IdP

  Admin->>ADS: POST /permission-sources
  Admin->>ADS: POST /permission-mappings
  Note over ADS: On login, broker passes IdP claims
  Note over ADS: Future: evaluate mappings → institutional_mapping grants
```

Institutional mapping evaluation at login is configured via `permission-sources` and
`permission-mappings`; grant creation from IdP claims can be extended in a broker callback hook.
