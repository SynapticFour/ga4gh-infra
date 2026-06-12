# ADS event model

ADS emits immutable audit events to the `audit_events` table and structured logs.

## Event types

| Event | Type string | When emitted |
|-------|-------------|--------------|
| Grant created | `grant.created` | DAC approval, DUO auto-approval, or institutional grant insert |
| Grant revoked | `grant.revoked` | `DELETE /grants/{id}` |
| Request created | `request.created` | `POST /access-requests` |
| Request approved | `request.approved` | DAC approve or DUO auto-approval |
| Request rejected | `request.rejected` | DAC reject |

## Payload shape

Each event is an `AdsEvent`:

```json
{
  "id": "uuid",
  "event_type": "grant.created",
  "occurred_at": "2026-06-12T12:00:00Z",
  "payload": {
    "grant_id": "uuid",
    "researcher_id": "researcher@example.org",
    "dataset_id": "uuid"
  }
}
```

Payload keys vary by event type:

- **grant.created** — `grant_id`, `researcher_id`, `dataset_id`
- **grant.revoked** — `grant_id`
- **request.created** — `request_id`, `researcher_id`
- **request.approved** / **request.rejected** — `request_id`

## Access decisions vs events

**AccessDecision** records (table `access_decisions`) capture DAC/system outcomes with actor
and reason — suitable for DACReS-style audit. **AdsEvent** records are lighter-weight
integration hooks for downstream SIEM, webhooks, or analytics (not yet exported via HTTP).

## Future extensions

- Webhook delivery on event insert
- CloudEvents envelope
- Correlation ids linking broker login, access request, and grant
