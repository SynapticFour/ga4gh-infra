// SPDX-License-Identifier: Apache-2.0

//! Peppered API-key hashing. Legacy unsalted SHA-256 hex is still accepted.

use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type HmacSha256 = Hmac<Sha256>;

const HMAC_PREFIX: &str = "hmac-sha256:";

fn hex_encode(bytes: impl AsRef<[u8]>) -> String {
    bytes
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sha256_hex(raw: &str) -> String {
    hex_encode(Sha256::digest(raw.as_bytes()))
}

fn hmac_hex(raw: &str, pepper: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(pepper.as_bytes()).expect("HMAC-SHA256 accepts any pepper");
    mac.update(raw.as_bytes());
    format!("{HMAC_PREFIX}{}", hex_encode(mac.finalize().into_bytes()))
}

/// Hash a raw API key for storage.
///
/// When `pepper` is empty this is unsalted SHA-256 (legacy). Otherwise the
/// stored value is `hmac-sha256:{hex}`.
pub fn hash_api_key(raw: &str, pepper: &str) -> String {
    if pepper.is_empty() {
        sha256_hex(raw)
    } else {
        hmac_hex(raw, pepper)
    }
}

/// Candidate hashes to look up for a presented key (current scheme + legacy).
pub fn lookup_hashes(raw: &str, pepper: &str) -> Vec<String> {
    let mut hashes = Vec::with_capacity(2);
    if !pepper.is_empty() {
        hashes.push(hmac_hex(raw, pepper));
    }
    let legacy = sha256_hex(raw);
    if hashes.first() != Some(&legacy) {
        hashes.push(legacy);
    }
    hashes
}

/// Constant-time compare of a presented key against a stored hash.
pub fn verify_api_key(raw: &str, stored_hash: &str, pepper: &str) -> bool {
    lookup_hashes(raw, pepper)
        .iter()
        .any(|candidate| constant_time_eq(candidate.as_bytes(), stored_hash.as_bytes()))
}

/// Constant-time equality for equal-length slices (length mismatch is `false`).
pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
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
    fn empty_pepper_is_sha256_hex() {
        let hash = hash_api_key("secret-key", "");
        assert_eq!(hash.len(), 64);
        assert!(!hash.starts_with(HMAC_PREFIX));
        assert!(verify_api_key("secret-key", &hash, ""));
        assert!(!verify_api_key("other", &hash, ""));
    }

    #[test]
    fn peppered_hash_is_versioned_hmac() {
        let hash = hash_api_key("secret-key", "institute-pepper");
        assert!(hash.starts_with(HMAC_PREFIX));
        assert!(verify_api_key("secret-key", &hash, "institute-pepper"));
        assert!(!verify_api_key("secret-key", &hash, "wrong-pepper"));
        assert!(!verify_api_key("other", &hash, "institute-pepper"));
    }

    #[test]
    fn lookup_accepts_legacy_sha256_after_pepper_is_enabled() {
        let legacy = hash_api_key("secret-key", "");
        assert!(verify_api_key("secret-key", &legacy, "institute-pepper"));
        let hashes = lookup_hashes("secret-key", "institute-pepper");
        assert_eq!(hashes.len(), 2);
        assert!(hashes.iter().any(|h| h == &legacy));
    }
}
