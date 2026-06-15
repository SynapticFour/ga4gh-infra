/// Escape a field for CSV output (RFC 4180-style quoting).
pub fn escape_field(value: &str) -> String {
    if value.contains(['"', ',', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

/// Join fields into one CSV row including trailing newline.
pub fn row(fields: &[&str]) -> String {
    let line = fields
        .iter()
        .map(|f| escape_field(f))
        .collect::<Vec<_>>()
        .join(",");
    format!("{line}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quotes_commas_and_newlines() {
        assert_eq!(escape_field("plain"), "plain");
        assert_eq!(escape_field("a,b"), "\"a,b\"");
        assert_eq!(escape_field("line\nbreak"), "\"line\nbreak\"");
    }
}
