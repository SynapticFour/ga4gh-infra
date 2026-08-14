// SPDX-License-Identifier: Apache-2.0

//! Test helpers shared across visa registry unit tests.

use std::sync::OnceLock;

use rand::SeedableRng;
use rsa::pkcs8::EncodePrivateKey;
use rsa::RsaPrivateKey;

use crate::keys::SigningKeys;

/// Return a deterministic PKCS#8 PEM for unit tests.
pub fn test_signing_key_pem() -> &'static str {
    static PEM: OnceLock<String> = OnceLock::new();
    PEM.get_or_init(|| {
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(99);
        let private_key =
            RsaPrivateKey::new(&mut rng, 2048).expect("generate deterministic test RSA key");
        private_key
            .to_pkcs8_pem(rsa::pkcs8::LineEnding::LF)
            .expect("encode test key")
            .to_string()
    })
}

/// Return deterministic signing keys for unit tests.
pub fn test_signing_keys() -> &'static SigningKeys {
    static KEYS: OnceLock<SigningKeys> = OnceLock::new();
    KEYS.get_or_init(|| SigningKeys::from_pem(test_signing_key_pem()).expect("test signing keys"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_test_keys() {
        let a = test_signing_keys().jwks().clone();
        let b = test_signing_keys().jwks();
        assert_eq!(a, *b);
    }
}
