# ga4gh-clearinghouse

A library for validating [GA4GH Passports and Visas](https://github.com/ga4gh-duri/ga4gh-duri.github.io/blob/master/researcher_ids/ga4gh_passport_v1.md) at resource-service boundaries.

Use this crate in DRS, TES, WES, Beacon, or any Rust service that must verify incoming Passport JWTs without depending on a particular HTTP framework.

## Usage

```toml
[dependencies]
ga4gh-clearinghouse = "0.1"

# Optional axum extractor for handlers:
ga4gh-clearinghouse = { version = "0.1", features = ["axum"] }
```

## Example

```rust
use std::time::Duration;
use ga4gh_clearinghouse::{
    Clearinghouse, ClearinghouseConfig, PolicyCheck, TrustedBroker,
};

let clearinghouse = Clearinghouse::new(ClearinghouseConfig::new(
    vec![TrustedBroker::new(
        "https://aai.example.org".to_string(),
        "https://aai.example.org/jwks.json".to_string(),
    )],
    Duration::from_secs(300),
))
.await?;

let passport = clearinghouse.validate_passport(raw_jwt).await?;
let visas = clearinghouse.extract_visas(&passport).await?;
let result = clearinghouse.check_policy(
    &visas,
    &PolicyCheck::HasControlledAccess {
        dataset_id: "dataset-123".to_string(),
    },
);
assert!(result.permitted);
```

## Features

| Feature | Description |
|---------|-------------|
| `axum` | `ExtractedPassport` extractor and `ClearinghouseState` trait for axum handlers |

## JWKS caching

The clearinghouse fetches and caches JWKS documents per trusted issuer. Cache entries expire after a configurable TTL and refresh automatically when a JWT references an unknown `kid`.

## License

Apache-2.0
