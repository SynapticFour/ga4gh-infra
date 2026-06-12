// SPDX-License-Identifier: Apache-2.0

//! Build script: fetch or bundle DUO OWL and emit static JSON for runtime.

use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use quick_xml::events::Event;
use quick_xml::Reader;
use serde::Serialize;

const DUO_OWL_URL: &str = "https://raw.githubusercontent.com/EBISPOT/DUO/master/duo.owl";
const PERMISSION_ROOT: &str = "DUO:0000001";
const MODIFIER_ROOT: &str = "DUO:0000017";

#[derive(Debug, Clone, Serialize)]
struct DuoTermJson {
    code: String,
    obo_id: String,
    label: String,
    definition: String,
    category: String,
    parents: Vec<String>,
    obsolete: bool,
}

#[derive(Debug, Clone, Serialize)]
struct DuoTermsDocument {
    source: String,
    terms: Vec<DuoTermJson>,
}

#[derive(Default, Clone)]
struct RawClass {
    obo_id: String,
    shorthand: Option<String>,
    label: Option<String>,
    definition: Option<String>,
    parents: Vec<String>,
    obsolete: bool,
}

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("out dir"));
    let bundled = manifest_dir.join("data/duo.owl");

    println!("cargo:rerun-if-changed=data/duo.owl");

    let (owl, source) = load_owl(&bundled);
    let classes = parse_owl_classes(&owl);
    let document = build_document(&classes, &source);
    let json = serde_json::to_string_pretty(&document).expect("serialize duo terms");
    fs::write(out_dir.join("duo_terms.json"), json).expect("write duo_terms.json");
}

fn load_owl(bundled: &Path) -> (String, String) {
    if let Ok(response) = reqwest::blocking::Client::builder()
        .use_rustls_tls()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .and_then(|client| client.get(DUO_OWL_URL).send())
    {
        if response.status().is_success() {
            if let Ok(body) = response.text() {
                return (body, DUO_OWL_URL.to_string());
            }
        }
    }

    let body = fs::read_to_string(bundled)
        .unwrap_or_else(|err| panic!("failed to fetch DUO OWL and bundled copy missing: {err}"));
    (body, bundled.display().to_string())
}

fn parse_owl_classes(owl: &str) -> Vec<RawClass> {
    let mut reader = Reader::from_str(owl);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut classes = Vec::new();
    let mut current: Option<RawClass> = None;
    let mut current_field = String::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(start)) | Ok(Event::Empty(start)) => {
                let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
                if name.ends_with("Class") {
                    if let Some(about) = attr_value(&start, "about") {
                        if let Some(obo_id) = uri_to_obo_id(&about) {
                            current = Some(RawClass {
                                obo_id,
                                ..Default::default()
                            });
                        }
                    }
                } else if current.is_some() {
                    current_field = local_name(&name).to_string();
                    if current_field == "subClassOf" {
                        if let Some(parent_uri) = attr_value(&start, "resource") {
                            if let Some(parent_id) = uri_to_obo_id(&parent_uri) {
                                if let Some(class) = current.as_mut() {
                                    class.parents.push(parent_id);
                                }
                            }
                        }
                    }
                }
            }
            Ok(Event::Text(text)) => {
                if let Some(class) = current.as_mut() {
                    let value = text.unescape().unwrap_or_default().trim().to_string();
                    if value.is_empty() {
                        continue;
                    }
                    match current_field.as_str() {
                        "shorthand" => class.shorthand = Some(value),
                        "label" => class.label = Some(value),
                        "IAO_0000115" | "comment" if class.definition.is_none() => {
                            class.definition = Some(value);
                        }
                        "deprecated" if value == "true" => class.obsolete = true,
                        _ => {}
                    }
                }
            }
            Ok(Event::End(end)) => {
                let name = String::from_utf8_lossy(end.name().as_ref()).into_owned();
                if name.ends_with("Class") {
                    if let Some(class) = current.take() {
                        if class.shorthand.is_some() {
                            classes.push(class);
                        }
                    }
                    current_field.clear();
                }
            }
            Ok(Event::Eof) => break,
            Err(err) => panic!("failed to parse DUO OWL: {err}"),
            _ => {}
        }
        buf.clear();
    }

    classes
}

fn build_document(classes: &[RawClass], source: &str) -> DuoTermsDocument {
    let by_id: HashMap<&str, &RawClass> = classes
        .iter()
        .map(|class| (class.obo_id.as_str(), class))
        .collect();

    let mut terms = Vec::new();
    for class in classes {
        let Some(code) = class.shorthand.as_ref() else {
            continue;
        };
        let ancestors = collect_ancestors(&class.obo_id, &by_id);
        let category = if ancestors.contains(PERMISSION_ROOT) {
            "permission"
        } else if ancestors.contains(MODIFIER_ROOT) {
            "modifier"
        } else {
            continue;
        };

        terms.push(DuoTermJson {
            code: code.clone(),
            obo_id: class.obo_id.clone(),
            label: class.label.clone().unwrap_or_else(|| code.clone()),
            definition: class.definition.clone().unwrap_or_default(),
            category: category.to_string(),
            parents: class.parents.clone(),
            obsolete: class.obsolete,
        });
    }

    terms.sort_by(|left, right| left.code.cmp(&right.code));

    DuoTermsDocument {
        source: source.to_string(),
        terms,
    }
}

fn collect_ancestors(obo_id: &str, by_id: &HashMap<&str, &RawClass>) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut stack = vec![obo_id.to_string()];
    while let Some(current) = stack.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        if let Some(class) = by_id.get(current.as_str()) {
            for parent in &class.parents {
                stack.push(parent.clone());
            }
        }
    }
    seen
}

fn attr_value(start: &quick_xml::events::BytesStart<'_>, name: &str) -> Option<String> {
    start
        .attributes()
        .flatten()
        .find(|attr| attr.key.as_ref().ends_with(name.as_bytes()))
        .and_then(|attr| String::from_utf8(attr.value.to_vec()).ok())
}

fn local_name(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn uri_to_obo_id(uri: &str) -> Option<String> {
    let fragment = uri.rsplit('/').next()?;
    if !fragment.starts_with("DUO_") {
        return None;
    }
    Some(fragment.replace('_', ":"))
}
