//! Parse the `PartAtom` OLE stream — plain Atom-format XML that Autodesk
//! uses to carry family metadata (title, category, taxonomy, links).
//!
//! Namespace: `urn:schemas-autodesk-com:partatom`

use crate::{Error, Result};
use quick_xml::Reader;
use quick_xml::events::Event;
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

        let mut atom = PartAtom {
            raw_xml: raw_xml.clone(),
            ..Default::default()
        };
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
                        "taxonomy" => {
                            current_taxonomy = Some(Taxonomy {
                                term: String::new(),
                                label: String::new(),
                            })
                        }
                        "term" => {
                            // Used in both <taxonomy> and <category>; the
                            // text handler routes the captured value
                            // based on whether a current_taxonomy is
                            // open. Scheme lives on the <category>
                            // parent — do NOT overwrite
                            // last_category_scheme here.
                            state = State::InTaxonomyTerm;
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
                                if term.chars().all(|c| c.is_ascii_digit() || c == '.')
                                    && term.contains('.')
                                {
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

    /// Encode a `PartAtom` back to Atom-format XML bytes (WRT-08).
    /// Inverse of [`Self::from_bytes`].
    ///
    /// The writer emits UTF-8 XML with the Autodesk schema
    /// declarations:
    ///
    /// ```xml
    /// <?xml version="1.0" encoding="UTF-8"?>
    /// <entry xmlns="http://www.w3.org/2005/Atom"
    ///        xmlns:A="urn:schemas-autodesk-com:partatom">
    ///   <title>...</title>
    ///   <id>...</id>
    ///   <updated>...</updated>
    ///   <A:taxonomy><term>...</term><label>...</label></A:taxonomy>
    ///   <category><term>...</term><scheme>...</scheme></category>
    /// </entry>
    /// ```
    ///
    /// Optional fields (`title`, `id`, `updated`, plus each taxonomy
    /// and category) are omitted when `None` / empty. Text content
    /// is XML-escaped so special characters (`<`, `>`, `&`, quotes)
    /// survive the round-trip. Emits one element per line with a
    /// two-space indent for diff-friendly output.
    ///
    /// Round-trip guarantee: `PartAtom::from_bytes(&atom.encode())`
    /// preserves `title`, `id`, `updated`, every `Taxonomy`, every
    /// `Category`, and `omniclass`. `raw_xml` will differ (it's the
    /// reconstructed text, not a byte-echo of the original input).
    pub fn encode(&self) -> Vec<u8> {
        let mut out = String::with_capacity(256);
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str(
            "<entry xmlns=\"http://www.w3.org/2005/Atom\" \
             xmlns:A=\"urn:schemas-autodesk-com:partatom\">\n",
        );
        if let Some(t) = self.title.as_deref() {
            out.push_str("  <title>");
            out.push_str(&xml_escape_text(t));
            out.push_str("</title>\n");
        }
        if let Some(id) = self.id.as_deref() {
            out.push_str("  <id>");
            out.push_str(&xml_escape_text(id));
            out.push_str("</id>\n");
        }
        if let Some(u) = self.updated.as_deref() {
            out.push_str("  <updated>");
            out.push_str(&xml_escape_text(u));
            out.push_str("</updated>\n");
        }
        for tax in &self.taxonomies {
            out.push_str("  <A:taxonomy><term>");
            out.push_str(&xml_escape_text(&tax.term));
            out.push_str("</term><label>");
            out.push_str(&xml_escape_text(&tax.label));
            out.push_str("</label></A:taxonomy>\n");
        }
        for cat in &self.categories {
            out.push_str("  <category");
            if let Some(scheme) = cat.scheme.as_deref() {
                out.push_str(" scheme=\"");
                out.push_str(&xml_escape_text(scheme));
                out.push('"');
            }
            out.push_str("><term>");
            out.push_str(&xml_escape_text(&cat.term));
            out.push_str("</term></category>\n");
        }
        out.push_str("</entry>\n");
        out.into_bytes()
    }
}

fn xml_escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
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

    // ---- WRT-08: PartAtom writer round-trip ----

    fn sample_atom() -> PartAtom {
        PartAtom {
            title: Some("racbasicsamplefamily".into()),
            id: Some("Table-End-0000-CAN-ENU".into()),
            updated: Some("2023-03-27T11:56:02Z".into()),
            taxonomies: vec![Taxonomy {
                term: "adsk:revit".into(),
                label: "Autodesk Revit".into(),
            }],
            categories: vec![
                Category {
                    term: "23.40.20.14.17".into(),
                    scheme: Some("std:oc1".into()),
                },
                Category {
                    term: "Furniture".into(),
                    scheme: Some("adsk:revit:grouping".into()),
                },
            ],
            omniclass: Some("23.40.20.14.17".into()),
            raw_xml: String::new(),
        }
    }

    #[test]
    fn encode_round_trips_full_sample() {
        let original = sample_atom();
        let bytes = original.encode();
        let decoded = PartAtom::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.title, original.title);
        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.updated, original.updated);
        assert_eq!(decoded.taxonomies.len(), original.taxonomies.len());
        assert_eq!(decoded.taxonomies[0].term, original.taxonomies[0].term);
        assert_eq!(decoded.taxonomies[0].label, original.taxonomies[0].label);
        assert_eq!(decoded.categories.len(), original.categories.len());
        assert_eq!(decoded.categories[0].term, original.categories[0].term);
        assert_eq!(decoded.categories[0].scheme, original.categories[0].scheme);
        assert_eq!(decoded.omniclass, original.omniclass);
    }

    #[test]
    fn encode_escapes_xml_special_chars() {
        let atom = PartAtom {
            title: Some("A & B <test>".into()),
            ..Default::default()
        };
        let bytes = atom.encode();
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.contains("A &amp; B &lt;test&gt;"));
        let decoded = PartAtom::from_bytes(&bytes).unwrap();
        assert_eq!(decoded.title.as_deref(), Some("A & B <test>"));
    }

    #[test]
    fn encode_omits_missing_optional_fields() {
        let atom = PartAtom {
            title: Some("minimal".into()),
            ..Default::default()
        };
        let bytes = atom.encode();
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.contains("<title>minimal</title>"));
        assert!(!s.contains("<id>"));
        assert!(!s.contains("<updated>"));
        assert!(!s.contains("<category>"));
        assert!(!s.contains("<A:taxonomy>"));
    }

    #[test]
    fn encode_emits_well_formed_xml_prolog() {
        let bytes = sample_atom().encode();
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(s.contains("xmlns=\"http://www.w3.org/2005/Atom\""));
        assert!(s.contains("xmlns:A=\"urn:schemas-autodesk-com:partatom\""));
        assert!(s.trim_end().ends_with("</entry>"));
    }

    #[test]
    fn encode_category_without_scheme_omits_scheme_tag() {
        let atom = PartAtom {
            categories: vec![Category {
                term: "Custom".into(),
                scheme: None,
            }],
            ..Default::default()
        };
        let bytes = atom.encode();
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.contains("<category><term>Custom</term></category>"));
        assert!(!s.contains("<scheme>"));
    }

    #[test]
    fn xml_escape_covers_all_five_reserved_chars() {
        let s = xml_escape_text("<>&\"'");
        assert_eq!(s, "&lt;&gt;&amp;&quot;&apos;");
    }

    #[test]
    fn empty_atom_encodes_valid_minimal_document() {
        let bytes = PartAtom::default().encode();
        let s = std::str::from_utf8(&bytes).unwrap();
        assert!(s.starts_with("<?xml"));
        assert!(s.contains("<entry"));
        assert!(s.contains("</entry>"));
        // Round-trips as an empty PartAtom.
        let decoded = PartAtom::from_bytes(&bytes).unwrap();
        assert!(decoded.title.is_none());
        assert!(decoded.taxonomies.is_empty());
        assert!(decoded.categories.is_empty());
    }
}
