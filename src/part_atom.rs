//! Parse the `PartAtom` OLE stream — plain Atom-format XML that Autodesk
//! uses to carry family metadata (title, category, taxonomy, links).
//!
//! Namespace: `urn:schemas-autodesk-com:partatom`

use crate::{Error, Result};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartAtom {
    pub title: Option<String>,
    pub id: Option<String>,
    pub updated: Option<String>,
    pub taxonomies: Vec<Taxonomy>,
    pub categories: Vec<Category>,
    /// Autodesk OmniClass code (e.g. `23.40.20.14.17` = Furniture).
    pub omniclass: Option<String>,
    /// Raw XML for lossless pass-through if a downstream tool wants it.
    pub raw_xml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Taxonomy {
    pub term: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub term: String,
    pub scheme: Option<String>,
}

impl PartAtom {
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let raw_xml = std::str::from_utf8(data)
            .map_err(|e| Error::PartAtom(format!("invalid UTF-8: {e}")))?
            .to_string();

        let mut atom = PartAtom { raw_xml: raw_xml.clone(), ..Default::default() };
        let mut reader = Reader::from_str(&raw_xml);
        reader.config_mut().trim_text(true);

        enum State {
            Top,
            InTitle,
            InId,
            InUpdated,
            InTaxonomyTerm,
            InTaxonomyLabel,
        }

        let mut state = State::Top;
        let mut buf = Vec::new();
        let mut current_taxonomy: Option<Taxonomy> = None;
        let mut last_category_term: Option<String> = None;
        let mut last_category_scheme: Option<String> = None;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name_owned = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let (prefix, local) = match name_owned.split_once(':') {
                        Some((p, l)) => (Some(p.to_string()), l.to_string()),
                        None => (None, name_owned.clone()),
                    };
                    let _ = prefix; // prefix unused for our subset
                    match local.as_str() {
                        "title" => state = State::InTitle,
                        "id" => state = State::InId,
                        "updated" => state = State::InUpdated,
                        "taxonomy" => current_taxonomy = Some(Taxonomy { term: String::new(), label: String::new() }),
                        "term" => {
                            if current_taxonomy.is_some() {
                                state = State::InTaxonomyTerm;
                            } else {
                                // category's term
                                state = State::InTaxonomyTerm;
                                last_category_scheme = e.attributes().find_map(|a| {
                                    let a = a.ok()?;
                                    if a.key.as_ref() == b"scheme" {
                                        Some(String::from_utf8_lossy(&a.value).to_string())
                                    } else {
                                        None
                                    }
                                });
                            }
                        }
                        "label" => state = State::InTaxonomyLabel,
                        "category" => {
                            last_category_scheme = e.attributes().find_map(|a| {
                                let a = a.ok()?;
                                if a.key.as_ref() == b"scheme" {
                                    Some(String::from_utf8_lossy(&a.value).to_string())
                                } else {
                                    None
                                }
                            });
                            last_category_term = None;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => {
                    let text = e.unescape().unwrap_or_default().trim().to_string();
                    if text.is_empty() {
                        continue;
                    }
                    match state {
                        State::InTitle => atom.title = Some(text),
                        State::InId => atom.id = Some(text),
                        State::InUpdated => atom.updated = Some(text),
                        State::InTaxonomyTerm => {
                            if let Some(t) = current_taxonomy.as_mut() {
                                t.term = text;
                            } else {
                                last_category_term = Some(text);
                            }
                        }
                        State::InTaxonomyLabel => {
                            if let Some(t) = current_taxonomy.as_mut() {
                                t.label = text;
                            }
                        }
                        State::Top => {}
                    }
                }
                Ok(Event::End(e)) => {
                    let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    let local = name.rsplit(':').next().unwrap_or(&name).to_string();
                    match local.as_str() {
                        "taxonomy" => {
                            if let Some(t) = current_taxonomy.take() {
                                atom.taxonomies.push(t);
                            }
                        }
                        "category" => {
                            if let Some(term) = last_category_term.take() {
                                // OmniClass codes are numeric dotted identifiers like "23.40.20.14.17"
                                if term.chars().all(|c| c.is_ascii_digit() || c == '.') && term.contains('.') {
                                    atom.omniclass.get_or_insert(term.clone());
                                }
                                atom.categories.push(Category {
                                    term,
                                    scheme: last_category_scheme.clone(),
                                });
                            }
                            last_category_scheme = None;
                        }
                        _ => {}
                    }
                    state = State::Top;
                }
                Ok(Event::Eof) => break,
                Err(e) => return Err(Error::PartAtom(format!("{e}"))),
                _ => {}
            }
            buf.clear();
        }

        Ok(atom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sample_partatom() {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
<entry xmlns="http://www.w3.org/2005/Atom" xmlns:A="urn:schemas-autodesk-com:partatom">
<title>racbasicsamplefamily</title>
<id>Table-End-0000-CAN-ENU</id>
<updated>2023-03-27T11:56:02Z</updated>
<A:taxonomy><term>adsk:revit</term><label>Autodesk Revit</label></A:taxonomy>
<category><term>23.40.20.14.17</term><scheme>std:oc1</scheme></category>
<category><term>Furniture</term><scheme>adsk:revit:grouping</scheme></category>
</entry>"#;
        let atom = PartAtom::from_bytes(xml.as_bytes()).unwrap();
        assert_eq!(atom.title.as_deref(), Some("racbasicsamplefamily"));
        assert_eq!(atom.omniclass.as_deref(), Some("23.40.20.14.17"));
        assert_eq!(atom.categories.len(), 2);
        assert_eq!(atom.taxonomies.len(), 1);
        assert_eq!(atom.taxonomies[0].label, "Autodesk Revit");
    }
}
