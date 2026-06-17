//! Links to ADS entities (datasets, projects) with friendly labels.

use std::collections::HashMap;

use ga4gh_types::{Dataset, ResearchProject};
use uuid::Uuid;

/// A resource shown as a link with optional secondary identifier (external ID).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntityRef {
    pub id: String,
    pub name: String,
    pub subtitle: Option<String>,
    pub href: String,
}

impl EntityRef {
    pub fn dataset(id: Uuid, datasets: &HashMap<Uuid, &Dataset>) -> Self {
        let href = format!("/datasets/{id}");
        if let Some(d) = datasets.get(&id) {
            Self {
                id: id.to_string(),
                name: d.name.clone(),
                subtitle: d.external_id.clone(),
                href,
            }
        } else {
            Self {
                id: id.to_string(),
                name: short_id(id),
                subtitle: None,
                href,
            }
        }
    }

    pub fn project(id: Uuid, projects: &HashMap<Uuid, &ResearchProject>) -> Self {
        let href = format!("/projects/{id}");
        if let Some(p) = projects.get(&id) {
            Self {
                id: id.to_string(),
                name: p.name.clone(),
                subtitle: Some(p.researcher_id.clone()),
                href,
            }
        } else {
            Self {
                id: id.to_string(),
                name: short_id(id),
                subtitle: None,
                href,
            }
        }
    }
}

fn short_id(id: Uuid) -> String {
    let s = id.to_string();
    if s.len() > 8 {
        format!("{}…", &s[..8])
    } else {
        s
    }
}

impl EntityRef {
    /// Render as a link with tooltip for Askama templates (`|safe`).
    pub fn html(&self) -> String {
        let subtitle = self
            .subtitle
            .as_ref()
            .map(|s| format!(r#"<span class="entity-subtitle">{}</span>"#, escape_html(s)))
            .unwrap_or_default();
        let tooltip_extra = self
            .subtitle
            .as_ref()
            .map(|s| format!(" · {}", s.replace('"', "&quot;")))
            .unwrap_or_default();
        format!(
            r#"<a href="{href}" class="entity-link" title="ID: {id}{tooltip_extra}">{name}{subtitle}</a>"#,
            href = escape_attr(&self.href),
            id = escape_attr(&self.id),
            tooltip_extra = tooltip_extra,
            name = escape_html(&self.name),
            subtitle = subtitle,
        )
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
