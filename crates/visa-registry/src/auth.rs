// SPDX-License-Identifier: Apache-2.0

//! DAC API key hashing and verification.

use sha2::{Digest, Sha256};

/// Hash a raw API key for storage in PostgreSQL.
pub fn hash_api_key(raw: &str) -> String {
    let digest = Sha256::digest(raw.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Compare a raw API key against a stored SHA-256 hex hash.
pub fn verify_api_key(raw: &str, stored_hash: &str) -> bool {
    let candidate = hash_api_key(raw);
    constant_time_eq(candidate.as_bytes(), stored_hash.as_bytes())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right.iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
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
}
