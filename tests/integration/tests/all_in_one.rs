//! Integration test launching the built `ga4gh-infra all-in-one` subprocess with SQLite config.

use std::process::Command;

#[tokio::test]
#[ignore = "requires release-built ga4gh-infra binary (run after cargo build -p ga4gh-infra-cli --release)"]
async fn all_in_one_binary_is_invocable() {
    let binary = std::env::var("GA4GH_INFRA_BIN").unwrap_or_else(|_| {
        format!(
            "{}/../../target/release/ga4gh-infra",
            env!("CARGO_MANIFEST_DIR")
        )
    });

    let output = Command::new(&binary)
        .arg("--help")
        .output()
        .expect("spawn ga4gh-infra");

    assert!(
        output.status.success(),
        "ga4gh-infra --help failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("all-in-one"));
    assert!(stdout.contains("keygen"));
}
