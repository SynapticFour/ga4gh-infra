// SPDX-License-Identifier: Apache-2.0

//! Internal registration API key verification.

/// Compare a presented API key against the configured registration secret.
pub fn verify_registration_key(presented: &str, expected: &str) -> bool {
    constant_time_eq(presented.as_bytes(), expected.as_bytes())
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
    fn accepts_matching_key() {
        assert!(verify_registration_key("secret", "secret"));
    }

    #[test]
    fn rejects_wrong_key() {
        assert!(!verify_registration_key("wrong", "secret"));
    }
}
