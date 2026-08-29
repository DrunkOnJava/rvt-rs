//! Document-scoped identity and source-span contracts (Phase 1).
//!
//! These types establish **fail-closed** identity scoping before any ES
//! remapping decoder exists. They do **not** claim that ES ElementId
//! remapping, UniqueId recovery from production corpora, or BIM
//! topology joins are solved.
//!
//! See `docs/research/unified-research-report.md` (§15, §30).

use serde::{Deserialize, Serialize};
use std::fmt;

/// Stable identity for one opened Revit document / file artifact.
///
/// `document_key` is an opaque session- or path-derived key so ElementIds
/// from two files are never accidentally compared. Optional `file_guid`
/// mirrors BasicFileInfo when present; absence is honest, not an error.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentIdentity {
    /// Opaque document scope key (path hash, session id, or test fixture id).
    pub document_key: String,
    /// BasicFileInfo GUID when recovered; `None` when absent or unknown.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_guid: Option<String>,
    /// Revit release year when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revit_year: Option<u32>,
}

impl DocumentIdentity {
    /// Build an identity from an opaque key alone (tests / synthetics).
    pub fn from_key(document_key: impl Into<String>) -> Self {
        Self {
            document_key: document_key.into(),
            file_guid: None,
            revit_year: None,
        }
    }

    /// Attach optional BasicFileInfo fields without inventing values.
    pub fn with_file_meta(mut self, file_guid: Option<String>, revit_year: Option<u32>) -> Self {
        self.file_guid = file_guid;
        self.revit_year = revit_year;
        self
    }
}

/// Revit UniqueId when known (36-char GUID string form).
///
/// Phase 1 stores the string as recovered/declared by an oracle or
/// fixture manifest. No on-disk UniqueId decoder is implied.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UniqueId(pub String);

impl UniqueId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for UniqueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Element reference scoped to a [`DocumentIdentity`].
///
/// ElementIds are **not** globally unique across files. Comparing two
/// [`ScopedElementRef`] values requires equal `document` keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ScopedElementRef {
    pub document: DocumentIdentity,
    /// Runtime ElementId (`u32`), when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub element_id: Option<u32>,
    /// UniqueId when known from oracle / fixture truth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unique_id: Option<UniqueId>,
}

impl ScopedElementRef {
    /// ElementId-only ref inside a document scope.
    pub fn from_element_id(document: DocumentIdentity, element_id: u32) -> Self {
        Self {
            document,
            element_id: Some(element_id),
            unique_id: None,
        }
    }

    /// True when both sides share the same document key and ElementId.
    ///
    /// Returns `false` when either ElementId is missing (fail closed).
    pub fn same_element_id(&self, other: &Self) -> bool {
        self.document.document_key == other.document.document_key
            && self.element_id.is_some()
            && self.element_id == other.element_id
    }

    /// Cross-document ElementId equality is meaningless — always false.
    pub fn element_id_comparable_with(&self, other: &Self) -> bool {
        self.document.document_key == other.document.document_key
            && self.element_id.is_some()
            && other.element_id.is_some()
    }
}

/// Byte range identity inside a named OLE stream (research localization).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceSpan {
    /// Stream path inside the CFB container (e.g. `Global/Latest`).
    pub stream: String,
    /// Inclusive start offset in the (de)compressed stream view used by
    /// the observation — callers must record which view in evidence notes.
    pub start: u64,
    /// Exclusive end offset.
    pub end: u64,
}

impl SourceSpan {
    pub fn new(stream: impl Into<String>, start: u64, end: u64) -> Option<Self> {
        if end < start {
            return None;
        }
        Some(Self {
            stream: stream.into(),
            start,
            end,
        })
    }

    pub fn len(&self) -> u64 {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Lightweight record identity hook for observations (not a full decoder).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecordIdentity {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema_tag: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub span: Option<SourceSpan>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_element_refs_require_same_document() {
        let a = ScopedElementRef::from_element_id(DocumentIdentity::from_key("doc-a"), 42);
        let b = ScopedElementRef::from_element_id(DocumentIdentity::from_key("doc-b"), 42);
        let a2 = ScopedElementRef::from_element_id(DocumentIdentity::from_key("doc-a"), 42);
        assert!(!a.element_id_comparable_with(&b));
        assert!(!a.same_element_id(&b));
        assert!(a.same_element_id(&a2));
    }

    #[test]
    fn source_span_rejects_inverted_range() {
        assert!(SourceSpan::new("Global/Latest", 10, 4).is_none());
        let span = SourceSpan::new("Global/Latest", 4, 10).expect("valid");
        assert_eq!(span.len(), 6);
    }

    #[test]
    fn identity_round_trips_json() {
        let doc = DocumentIdentity::from_key("fixture:es-remap-00").with_file_meta(
            Some("d713e470-abcd-4321-9876-123456789012".into()),
            Some(2024),
        );
        let refer = ScopedElementRef {
            document: doc,
            element_id: Some(7),
            unique_id: Some(UniqueId("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee".into())),
        };
        let json = serde_json::to_value(&refer).expect("serialize");
        let back: ScopedElementRef = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, refer);
    }
}
