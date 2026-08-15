// SPDX-License-Identifier: Apache-2.0

//! DAC API key hashing and verification.

pub fn hash_api_key(raw: &str) -> String {
    ga4gh_http::hash_api_key(raw, "")
}

pub fn verify_api_key(raw: &str, stored_hash: &str) -> bool {
    ga4gh_http::verify_api_key(raw, stored_hash, "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_is_deterministic() {
        assert_eq!(hash_api_key("secret-key"), hash_api_key("secret-key"));
    }

    #[test]
    fn verify_accepts_matching_key() {
        let raw = "dac-admin-key";
        let hash = hash_api_key(raw);
        assert!(verify_api_key(raw, &hash));
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let hash = hash_api_key("correct-key");
        assert!(!verify_api_key("wrong-key", &hash));
    }

    #[test]
    fn peppered_hash_does_not_match_empty_pepper_verify() {
        let hash = ga4gh_http::hash_api_key("secret-key", "pepper");
        assert!(!verify_api_key("secret-key", &hash));
        assert!(ga4gh_http::verify_api_key("secret-key", &hash, "pepper"));
    }
}
