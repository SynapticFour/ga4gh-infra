# ga4gh-types

Shared [GA4GH](https://www.ga4gh.org/) data structures with full serde support.

This crate provides types aligned with:

- GA4GH Passport & Visa specification v1.2
- GA4GH Service Info v1.0.0
- Data Use Ontology (DUO) term codes

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
ga4gh-types = "0.1"
```

## Types

| Type | Purpose |
|------|---------|
| `Passport` | Decoded GA4GH Passport with embedded visa JWT strings |
| `Visa` / `VisaClaim` | Decoded visa JWT payload and `ga4gh_visa_v1` claim |
| `VisaType` | Standard visa type identifiers |
| `DuoCode` | DUO permission codes with `Display`, `FromStr`, and serde |
| `ServiceInfo` / `ServiceType` | GA4GH service metadata |

## Example

```rust
use ga4gh_types::{Passport, VisaType, DuoCode};

let passport: Passport = serde_json::from_str(json)?;
assert_eq!(passport.sub, "researcher@example.org");

let code = DuoCode::from_str("GRU")?;
assert_eq!(code.to_string(), "GRU");
```

## License

Apache-2.0
