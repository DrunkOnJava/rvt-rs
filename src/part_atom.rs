//! Parse the `PartAtom` OLE stream — plain Atom-format XML that Autodesk
//! uses to carry family metadata (title, category, taxonomy, links).
//!
//! Namespace: `urn:schemas-autodesk-com:partatom`

use crate::{Error, Result};
use quick_xml::Reader;
use quick_xml::XmlVersion;
use quick_xml::escape::resolve_predefined_entity;
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PartAtom {
    /// The last title-like element seen — on families the family type
    /// name (`0610 x 0915mm` on the reference sample), on project files a
    /// Project Information parameter-group name. Kept for compatibility;
    /// prefer [`Self::entry_title`] for the document's name.
    pub title: Option<String>,
    /// The Atom entry's own `<title>` (the first un-prefixed one): the
    /// document name Revit recorded, e.g. `racbasicsamplefamily`.
    #[serde(default)]
    pub entry_title: Option<String>,
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

/// One type of a family as the stream's type table lists it (`<A:family>`,
/// one `<A:part>` per type): the text Revit wrote of the type and its type
/// parameters when the family was saved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct FamilyType {
    /// The type's name (`0610 x 0915mm`).
    pub name: String,
    pub parameters: Vec<TypeParameter>,
}

/// One type parameter of a [`FamilyType`]. Revit writes no element for a
/// parameter whose value is empty.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TypeParameter {
    /// The element's name in the table (`Default_Elevation`).
    pub name: String,
    /// `displayName` (`Default Elevation`).
    pub display_name: Option<String>,
    /// `type`: `system` for a built-in parameter, `custom` for one the
    /// family defines, `shared` for a shared parameter.
    pub kind: Option<String>,
    /// `typeOfParameter` (`Length`, `Material`, `Text`, ...).
    pub type_of_parameter: Option<String>,
    /// `units` (`mm`), which an imperial family's lengths do not carry.
    pub units: Option<String>,
    /// The value as Revit printed it: `762.00` in `units`, or feet and
    /// inches (`2' - 0"`) on an imperial family.
    pub value: String,
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
        // Do not enable trim_text: quick-xml ≥0.38 streams entities as
        // separate `GeneralRef` events between `Text` fragments, and
        // per-fragment trimming would eat spaces adjacent to `&amp;` etc.
        // We trim the accumulated value once when flushing a field.

        enum State {
            Top,
            InTitle,
            InEntryTitle,
            InId,
            InUpdated,
            InTaxonomyTerm,
            InTaxonomyLabel,
        }

        let mut state = State::Top;
        let mut buf = Vec::new();
        let mut pending = String::new();
        let mut current_taxonomy: Option<Taxonomy> = None;
        let mut last_category_term: Option<String> = None;
        let mut last_category_scheme: Option<String> = None;

        // quick-xml 0.38+ no longer expands entities inside `Text`; they
        // arrive as `Event::GeneralRef`. Accumulate both, then flush on
        // the closing tag (matches prior `BytesText::unescape` behavior).
        let flush_pending = |pending: &mut String,
                             state: &State,
                             atom: &mut PartAtom,
                             current_taxonomy: &mut Option<Taxonomy>,
                             last_category_term: &mut Option<String>| {
            let text = std::mem::take(pending);
            let text = text.trim();
            if text.is_empty() {
                return;
            }
            let text = text.to_string();
            match state {
                State::InTitle => atom.title = Some(text),
                State::InEntryTitle => {
                    atom.entry_title = Some(text.clone());
                    atom.title = Some(text);
                }
                State::InId => atom.id = Some(text),
                State::InUpdated => atom.updated = Some(text),
                State::InTaxonomyTerm => {
                    if let Some(t) = current_taxonomy.as_mut() {
                        t.term = text;
                    } else {
                        *last_category_term = Some(text);
                    }
                }
                State::InTaxonomyLabel => {
                    if let Some(t) = current_taxonomy.as_mut() {
                        t.label = text;
                    }
                }
                State::Top => {}
            }
        };

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    let name_owned = e.name().into_inner().to_string();
                    let (prefix, local) = match name_owned.split_once(':') {
                        Some((p, l)) => (Some(p.to_string()), l.to_string()),
                        None => (None, name_owned.clone()),
                    };
                    match local.as_str() {
                        "title" if prefix.is_none() && atom.entry_title.is_none() => {
                            state = State::InEntryTitle
                        }
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
                                if a.key.into_inner() == "scheme" {
                                    a.normalized_value(XmlVersion::Implicit1_0)
                                        .ok()
                                        .map(|v| v.into_owned())
                                } else {
                                    None
                                }
                            });
                            last_category_term = None;
                        }
                        _ => {}
                    }
                }
                Ok(Event::Text(e)) => pending.push_str(&e),
                Ok(Event::GeneralRef(e)) => {
                    if let Ok(Some(ch)) = e.resolve_char_ref() {
                        pending.push(ch);
                    } else if let Some(val) = resolve_predefined_entity(&e) {
                        pending.push_str(val);
                    }
                }
                Ok(Event::End(e)) => {
                    flush_pending(
                        &mut pending,
                        &state,
                        &mut atom,
                        &mut current_taxonomy,
                        &mut last_category_term,
                    );
                    let name = e.name().into_inner();
                    let local = name.rsplit(':').next().unwrap_or(name).to_string();
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

    /// The family's types from the stream's type table, in the order the
    /// stream lists them; empty when it has none, as a project's
    /// `ProjectInformation` has none.
    pub fn family_types(&self) -> Result<Vec<FamilyType>> {
        family_types(&self.raw_xml)
    }

    /// Encode a `PartAtom` back to Atom-format XML bytes.
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
    // Tracker: WRT-08.
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

fn family_types(xml: &str) -> Result<Vec<FamilyType>> {
    fn local(name: &str) -> &str {
        name.rsplit(':').next().unwrap_or(name)
    }
    fn attribute(e: &quick_xml::events::BytesStart<'_>, key: &str) -> Option<String> {
        e.attributes().find_map(|a| {
            let a = a.ok()?;
            if a.key.into_inner() != key {
                return None;
            }
            a.normalized_value(XmlVersion::Implicit1_0)
                .ok()
                .map(|v| v.into_owned())
        })
    }
    let mut reader = Reader::from_str(xml);
    let mut buf = Vec::new();
    let mut types = Vec::new();
    // The depth of the open `<A:family>` and of the open `<A:part>`, and
    // the element open directly inside that part.
    let mut depth = 0usize;
    let mut family: Option<usize> = None;
    let mut part: Option<(usize, FamilyType)> = None;
    let mut child: Option<TypeParameter> = None;
    let mut text = String::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                depth += 1;
                let name = local(e.name().into_inner());
                let part_depth = part.as_ref().map(|(p, _)| *p);
                match (family, part_depth) {
                    (None, _) if name == "family" => family = Some(depth),
                    (Some(f), None) if depth == f + 1 && name == "part" => {
                        part = Some((
                            depth,
                            FamilyType {
                                name: String::new(),
                                parameters: Vec::new(),
                            },
                        ));
                    }
                    (_, Some(p)) if depth == p + 1 => {
                        text.clear();
                        child = Some(TypeParameter {
                            name: name.to_string(),
                            display_name: attribute(e, "displayName"),
                            kind: attribute(e, "type"),
                            type_of_parameter: attribute(e, "typeOfParameter"),
                            units: attribute(e, "units"),
                            value: String::new(),
                        });
                    }
                    _ => {}
                }
            }
            Ok(Event::Text(e)) if child.is_some() => text.push_str(&e),
            Ok(Event::GeneralRef(e)) if child.is_some() => {
                if let Ok(Some(ch)) = e.resolve_char_ref() {
                    text.push(ch);
                } else if let Some(val) = resolve_predefined_entity(&e) {
                    text.push_str(val);
                }
            }
            Ok(Event::End(_)) => {
                let part_depth = part.as_ref().map(|(p, _)| *p);
                match part_depth {
                    Some(p) if depth == p + 1 => {
                        if let (Some((_, current)), Some(mut parameter)) =
                            (part.as_mut(), child.take())
                        {
                            let value = text.trim().to_string();
                            if parameter.name == "title" {
                                current.name = value;
                            } else {
                                parameter.value = value;
                                current.parameters.push(parameter);
                            }
                        }
                    }
                    Some(p) if depth == p => {
                        if let Some((_, done)) = part.take() {
                            types.push(done);
                        }
                    }
                    _ => {}
                }
                if family == Some(depth) {
                    family = None;
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Err(e) => return Err(Error::PartAtom(format!("{e}"))),
            _ => {}
        }
        buf.clear();
    }
    Ok(types)
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
