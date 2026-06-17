use crate::clients::DuoTermOption;

/// Human-readable label for a DUO term code (subset used in tests and fallbacks).
pub fn duo_label(code: &str) -> String {
    match code {
        "DUO:0000006" => "Health or medical research".to_string(),
        "DUO:0000007" => "Disease-specific research".to_string(),
        "DUO:0000025" => "General research use".to_string(),
        "DUO:0000027" => "Research use only".to_string(),
        "DUO:0000042" => "No restriction".to_string(),
        other => other.to_string(),
    }
}

/// Resolve a DUO code or OBO id to label and CURIE using duo-service term catalog.
pub fn duo_display(code: &str, terms: &[DuoTermOption]) -> DuoDisplay {
    if let Some(term) = terms
        .iter()
        .find(|t| t.code.eq_ignore_ascii_case(code) || t.obo_id == code)
    {
        return DuoDisplay {
            label: term.label.clone(),
            curie: term.obo_id.clone(),
            definition: term.definition.clone(),
        };
    }
    DuoDisplay {
        label: duo_label(code),
        curie: code.to_string(),
        definition: String::new(),
    }
}

#[derive(Debug, Clone)]
pub struct DuoDisplay {
    pub label: String,
    pub curie: String,
    pub definition: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_code_returns_label() {
        assert_eq!(duo_label("DUO:0000006"), "Health or medical research");
    }

    #[test]
    fn unknown_code_passthrough() {
        assert_eq!(duo_label("DUO:9999999"), "DUO:9999999");
    }

    #[test]
    fn duo_display_uses_term_catalog() {
        let terms = vec![DuoTermOption {
            code: "GRU".into(),
            obo_id: "DUO:0000042".into(),
            label: "General research use".into(),
            definition: "Unrestricted research".into(),
            category: "permission".into(),
            obsolete: false,
        }];
        let display = duo_display("GRU", &terms);
        assert_eq!(display.label, "General research use");
        assert_eq!(display.curie, "DUO:0000042");
    }
}
