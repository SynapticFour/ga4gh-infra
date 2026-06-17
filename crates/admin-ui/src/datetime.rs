//! UTC timestamps for scientific / research portals (locale-neutral, ISO-based).

use chrono::{DateTime, Utc};

/// A UTC instant formatted for display with a machine-readable tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormattedDateTime {
    /// Human-readable UTC, e.g. `2026-06-17 18:55 UTC`.
    pub display: String,
    /// Full RFC 3339 for `<time datetime>` and tooltips.
    pub iso: String,
}

impl FormattedDateTime {
    /// Format an instant for tables and detail views.
    pub fn from_utc(dt: DateTime<Utc>) -> Self {
        Self {
            display: dt.format("%Y-%m-%d %H:%M UTC").to_string(),
            iso: dt.to_rfc3339(),
        }
    }

    /// Map optional timestamps; missing values render as an em dash without `<time>`.
    pub fn optional(dt: Option<DateTime<Utc>>) -> Self {
        dt.map(Self::from_utc).unwrap_or_else(Self::missing)
    }

    pub fn missing() -> Self {
        Self {
            display: "—".to_string(),
            iso: String::new(),
        }
    }

    pub fn has_iso(&self) -> bool {
        !self.iso.is_empty()
    }

    /// Render as `<time datetime="…">` for Askama templates (`|safe`).
    pub fn html(&self) -> String {
        if self.has_iso() {
            format!(
                r#"<time datetime="{iso}" title="{iso}">{display}</time>"#,
                iso = escape_attr(&self.iso),
                display = escape_html(&self.display),
            )
        } else {
            escape_html(&self.display)
        }
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    escape_html(s).replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn scientific_format_uses_utc_suffix() {
        let dt = Utc.with_ymd_and_hms(2026, 6, 17, 18, 55, 0).unwrap();
        let formatted = FormattedDateTime::from_utc(dt);
        assert_eq!(formatted.display, "2026-06-17 18:55 UTC");
        assert!(formatted.iso.contains("2026-06-17"));
    }

    #[test]
    fn optional_missing_renders_dash() {
        let formatted = FormattedDateTime::optional(None);
        assert_eq!(formatted.display, "—");
        assert!(!formatted.has_iso());
    }
}
