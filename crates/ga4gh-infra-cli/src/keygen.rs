// SPDX-License-Identifier: Apache-2.0

//! RS256 PEM key generation for broker and visa-registry signing.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use rand::thread_rng;
use rsa::pkcs8::{EncodePrivateKey, LineEnding};
use rsa::RsaPrivateKey;

/// Generate a new RS256 private key and write PKCS#8 PEM to `output`.
pub fn generate_pem(output: &Path, bits: usize) -> Result<()> {
    if bits < 2048 {
        anyhow::bail!("RSA key size must be at least 2048 bits");
    }

    if let Some(parent) = output.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .with_context(|| format!("creating directory {}", parent.display()))?;
        }
    }

    if output.exists() {
        anyhow::bail!("refusing to overwrite existing key at {}", output.display());
    }

    let mut rng = thread_rng();
    let private_key = RsaPrivateKey::new(&mut rng, bits).context("generating RSA private key")?;
    let pem = private_key
        .to_pkcs8_pem(LineEnding::LF)
        .context("encoding PKCS#8 PEM")?;

    fs::write(output, pem.as_bytes())
        .with_context(|| format!("writing key to {}", output.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(output, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("setting permissions on {}", output.display()))?;
    }

    Ok(())
}

/// Default development key filenames written by `keygen --output-dir`.
pub const BROKER_KEY_NAME: &str = "broker_rs256.pem";
pub const REGISTRY_KEY_NAME: &str = "registry_rs256.pem";

/// Generate default broker and visa-registry keys in `output_dir` when missing.
pub fn generate_default_keys(output_dir: &Path, bits: usize) -> Result<Vec<PathBuf>> {
    fs::create_dir_all(output_dir)
        .with_context(|| format!("creating directory {}", output_dir.display()))?;

    let mut written = Vec::new();
    for name in [BROKER_KEY_NAME, REGISTRY_KEY_NAME] {
        let path = output_dir.join(name);
        if path.exists() {
            continue;
        }
        generate_pem(&path, bits)?;
        written.push(path);
    }
    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rsa::pkcs8::DecodePrivateKey;

    #[test]
    fn writes_pkcs8_pem() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.pem");
        generate_pem(&path, 2048).expect("generate");
        let pem = fs::read_to_string(&path).expect("read");
        assert!(pem.contains("BEGIN PRIVATE KEY"));
        RsaPrivateKey::from_pkcs8_pem(&pem).expect("parse pem");
    }

    #[test]
    fn refuses_to_overwrite_existing_key() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.pem");
        generate_pem(&path, 2048).expect("first");
        assert!(generate_pem(&path, 2048).is_err());
    }

    #[test]
    fn generate_default_keys_skips_existing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let first = generate_default_keys(dir.path(), 2048).expect("first");
        assert_eq!(first.len(), 2);
        let second = generate_default_keys(dir.path(), 2048).expect("second");
        assert!(second.is_empty());
    }
}
