/// Returns true when the user holds the configured admin group/claim value.
pub fn is_admin(groups: &[String], admin_claim_value: &str) -> bool {
    groups.iter().any(|g| g == admin_claim_value)
}

/// DAC groups for operator-scoped ADS queries.
///
/// Admins receive `None` (unfiltered). Non-admins receive the intersection of
/// their IdP groups with the configured DAC allowlist (never unrestricted).
pub fn operator_dac_groups(
    session: &crate::session::UserSession,
    admin_claim_value: &str,
    allowed_dac_groups: &[String],
) -> Option<Vec<String>> {
    if session.is_admin {
        return None;
    }
    let groups: Vec<String> = session
        .groups
        .iter()
        .filter(|g| {
            *g != admin_claim_value && allowed_dac_groups.iter().any(|allowed| allowed == *g)
        })
        .cloned()
        .collect();
    Some(groups)
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
        assert!(operator_dac_groups(&s, "ga4gh-infra-admins", &["ega-dac".into()]).is_none());
    }

    #[test]
    fn operator_dac_groups_intersect_allowlist() {
        let s = session(&["ega-dac", "staff", "ga4gh-infra-admins"], false);
        assert_eq!(
            operator_dac_groups(&s, "ga4gh-infra-admins", &["ega-dac".into()]),
            Some(vec!["ega-dac".into()])
        );
    }

    #[test]
    fn staff_group_is_not_a_dac_operator() {
        let s = session(&["staff"], false);
        assert_eq!(
            operator_dac_groups(&s, "ga4gh-infra-admins", &["ega-dac".into()]),
            Some(Vec::<String>::new())
        );
    }

    #[test]
    fn operator_without_groups_is_empty_filter_not_unrestricted() {
        let s = session(&[], false);
        assert_eq!(
            operator_dac_groups(&s, "ga4gh-infra-admins", &["ega-dac".into()]),
            Some(Vec::<String>::new())
        );
    }
}
