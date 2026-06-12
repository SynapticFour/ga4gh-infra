// SPDX-License-Identifier: Apache-2.0

//! Dataset lookup and intended-use header parsing.

use axum::http::HeaderMap;

use crate::config::DatasetConfig;
use crate::error::SampleResourceError;

/// Parse a comma-separated intended-use header value.
pub fn parse_intended_use_header(headers: &HeaderMap) -> Option<Vec<String>> {
    headers
        .get("x-ga4gh-intended-use")
        .and_then(|value| value.to_str().ok())
        .map(parse_code_list)
}

/// Resolve intended-use codes from the request header or dataset defaults.
pub fn resolve_intended_use(
    headers: &HeaderMap,
    dataset: &DatasetConfig,
) -> Result<Vec<String>, SampleResourceError> {
    if let Some(codes) = parse_intended_use_header(headers) {
        if codes.is_empty() {
            return Err(SampleResourceError::BadRequest(
                "X-GA4GH-Intended-Use must not be empty".to_string(),
            ));
        }
        return Ok(codes);
    }

    if dataset.default_intended_use.is_empty() {
        return Err(SampleResourceError::Config(
            "dataset has no default_intended_use and caller omitted X-GA4GH-Intended-Use"
                .to_string(),
        ));
    }

    Ok(dataset.default_intended_use.clone())
}

fn parse_code_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    #[test]
    fn parses_intended_use_header() {
        let mut headers = HeaderMap::new();
        headers.insert("x-ga4gh-intended-use", HeaderValue::from_static("HMB, NPU"));

        assert_eq!(
            parse_intended_use_header(&headers),
            Some(vec!["HMB".to_string(), "NPU".to_string()])
        );
    }

    #[test]
    fn falls_back_to_dataset_defaults() {
        let headers = HeaderMap::new();
        let dataset = DatasetConfig {
            id: "demo".to_string(),
            name: "Demo".to_string(),
            description: None,
            duo: vec!["GRU".to_string()],
            default_intended_use: vec!["HMB".to_string()],
        };

        assert_eq!(
            resolve_intended_use(&headers, &dataset).expect("defaults"),
            vec!["HMB".to_string()]
        );
    }
}
