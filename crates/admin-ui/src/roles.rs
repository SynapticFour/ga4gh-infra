/// Returns true when the user holds the configured admin group/claim value.
pub fn is_admin(groups: &[String], admin_claim_value: &str) -> bool {
    groups.iter().any(|g| g == admin_claim_value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operator_without_admin_group() {
        assert!(!is_admin(&["operators".into()], "ga4gh-infra-admins"));
    }

    #[test]
    fn admin_with_matching_group() {
        assert!(is_admin(
            &["operators".into(), "ga4gh-infra-admins".into()],
            "ga4gh-infra-admins"
        ));
    }
}
