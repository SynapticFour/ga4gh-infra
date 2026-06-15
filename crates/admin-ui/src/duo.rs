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
}
