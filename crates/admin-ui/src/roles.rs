/// Returns true when the user holds the configured admin group/claim value.
pub fn is_admin(groups: &[String], admin_claim_value: &str) -> bool {
    groups.iter().any(|g| g == admin_claim_value)
}

/// DAC groups for operator-scoped ADS queries. Admins receive `None` (no filter).
pub fn operator_dac_groups(
    session: &crate::session::UserSession,
    admin_claim_value: &str,
) -> Option<Vec<String>> {
    if session.is_admin {
        return None;
    }
    let groups: Vec<String> = session
        .groups
        .iter()
        .filter(|g| *g != admin_claim_value)
        .cloned()
        .collect();
    if groups.is_empty() {
        None
    } else {
        Some(groups)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::UserSession;

    fn session(groups: &[&str], is_admin: bool) -> UserSession {
        UserSession {
            sub: "sub".into(),
            display_name: None,
            email: None,
            groups: groups.iter().map(|s| (*s).to_string()).collect(),
            is_admin,
            exp: 0,
        }
    }

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

    #[test]
    fn admin_has_no_dac_group_filter() {
        let s = session(&["ga4gh-infra-admins"], true);
        assert!(operator_dac_groups(&s, "ga4gh-infra-admins").is_none());
    }

    #[test]
    fn operator_dac_groups_exclude_admin_claim() {
        let s = session(&["ega-dac", "ga4gh-infra-admins"], false);
        assert_eq!(
            operator_dac_groups(&s, "ga4gh-infra-admins"),
            Some(vec!["ega-dac".into()])
        );
    }
}
