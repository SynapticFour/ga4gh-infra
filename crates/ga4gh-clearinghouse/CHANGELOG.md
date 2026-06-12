# Changelog

All notable changes to `ga4gh-clearinghouse` will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-06-12

### Added

- Passport and Visa JWT validation with JWKS fetching and in-memory cache refresh.
- Policy engine with controlled-access, affiliation, and combinator checks (`All` / `Any`).
- Optional `axum` feature providing an `ExtractedPassport` request extractor.

[Unreleased]: https://github.com/SynapticFour/ga4gh-infra/compare/ga4gh-clearinghouse-v0.1.0...HEAD
[0.1.0]: https://github.com/SynapticFour/ga4gh-infra/releases/tag/ga4gh-clearinghouse-v0.1.0
