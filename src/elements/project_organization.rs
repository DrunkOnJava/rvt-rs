//! `Phase` + `DesignOption` + `Workset` — the project-level
//! organization classes that slice the element set along non-spatial
//! axes. None of them map to a specific IFC4 entity; they surface as
//! property-set tags on the elements they group.
//!
//! - **Phase**: temporal classification. Elements are tagged with
//!   `phase_created` and `phase_demolished` so contractors can answer
//!   "what walls exist at phase 2?" without carrying two models.
//! - **DesignOption**: alternative-design classification. An element
//!   may exist in "Option A" but not "Option B" — when the user picks
//!   Option A as primary, B's elements are hidden.
//! - **Workset**: collaboration unit in workshared models — which
//!   team member "owns" a given element.
//!
//! # Typical field shape
//!
//! Phase:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Existing", "Phase 1", "New Construction" |
//! | `m_description` | String | Optional descriptive text |
//! | `m_sequence_number` | Primitive u32 | Ordering index (0 = earliest) |
//!
//! DesignOption:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Option 1", "Facade Study B" |
//! | `m_is_primary` | Primitive bool | True for the option shown by default |
//! | `m_option_set_id` | ElementId | Grouping set containing related options |
//!
//! Workset:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Architecture", "Mech/Plumb", "Shared Levels and Grids" |
//! | `m_unique_id` | String | Stable GUID for round-trip |
//! | `m_is_open` | Primitive bool | Whether the workset is loaded in the current view |
//! | `m_is_editable` | Primitive bool | Read-only vs writable |

use super::level::normalise_field_name;
use crate::formats;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

macro_rules! simple_decoder {
    ($Struct:ident, $name:literal) => {
        pub struct $Struct;

        impl ElementDecoder for $Struct {
            fn class_name(&self) -> &'static str {
                $name
            }

            fn decode(
                &self,
                bytes: &[u8],
                schema: &formats::ClassEntry,
                _index: &HandleIndex,
            ) -> Result<DecodedElement> {
                if schema.name != $name {
                    return Err(Error::BasicFileInfo(format!(
                        "{} received wrong schema: {}",
                        stringify!($Struct),
                        schema.name
                    )));
                }
                Ok(crate::walker::decode_instance(bytes, 0, schema))
            }
        }
    };
}

simple_decoder!(PhaseDecoder, "Phase");
simple_decoder!(DesignOptionDecoder, "DesignOption");
simple_decoder!(WorksetDecoder, "Workset");
simple_decoder!(RevisionDecoder, "Revision");

/// Typed view for Phase.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Phase {
    pub name: Option<String>,
    pub description: Option<String>,
    pub sequence_number: Option<u32>,
}

impl Phase {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("description", InstanceField::String(s)) => {
                    out.description = Some(s.clone());
                }
                ("sequencenumber" | "sequence", InstanceField::Integer { value, .. }) => {
                    out.sequence_number = Some(*value as u32);
                }
                _ => {}
            }
        }
        out
    }
}

/// Typed view for DesignOption.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DesignOption {
    pub name: Option<String>,
    pub is_primary: Option<bool>,
    pub option_set_id: Option<u32>,
}

impl DesignOption {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("isprimary" | "primary", InstanceField::Bool(b)) => {
                    out.is_primary = Some(*b);
                }
                ("optionsetid" | "setid", InstanceField::ElementId { id, .. }) => {
                    out.option_set_id = Some(*id);
                }
                _ => {}
            }
        }
        out
    }
}

/// Typed view for Workset.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Workset {
    pub name: Option<String>,
    pub unique_id: Option<String>,
    pub is_open: Option<bool>,
    pub is_editable: Option<bool>,
}

impl Workset {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("uniqueid" | "guid", InstanceField::String(s)) => {
                    out.unique_id = Some(s.clone());
                }
                ("isopen", InstanceField::Bool(b)) => out.is_open = Some(*b),
                ("iseditable" | "editable", InstanceField::Bool(b)) => {
                    out.is_editable = Some(*b);
                }
                _ => {}
            }
        }
        out
    }

    /// True when the workset is both open and editable — the typical
    /// "I can modify this" combination in workshared Revit models.
    pub fn is_modifiable(&self) -> Option<bool> {
        Some(self.is_open? && self.is_editable?)
    }
}

/// Issued-status enum for a Revit drawing revision.
///
/// Revit's `RevisionVisibility` + `RevisionIssued` split into a single
/// simplified enum. `Draft` means the revision is still being edited;
/// `Issued` means it has been signed off and the revision table on the
/// sheet should lock it in place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RevisionStatus {
    #[default]
    Draft,
    Issued,
    Other,
}

impl RevisionStatus {
    pub fn from_code(code: u32) -> Self {
        match code {
            0 => Self::Draft,
            1 => Self::Issued,
            _ => Self::Other,
        }
    }
}

/// Typed view of a decoded Revision.
///
/// Revisions are Revit's mechanism for tracking drawing-set updates
/// over a project's lifecycle. Each Revision gets a sequence number,
/// a date, a description, and an issued-to / issued-by pair. Sheets
/// display the active revisions in a revision schedule, and the
/// revision cloud tool annotates which regions of a drawing changed.
///
/// Typical fields (observed 2016–2026):
///
/// | Field | Type | Semantics |
/// |---|---|---|
/// | `m_sequence_number` | Primitive u32 | 1-indexed order; drives the default label. |
/// | `m_revision_date` | String | Free-form date, typically "YYYY-MM-DD" or "MM/DD/YY". |
/// | `m_description` | String | Human-readable summary, shown on the revision schedule. |
/// | `m_issued_to` | String | "Client", "Architect of Record", etc. |
/// | `m_issued_by` | String | Author / firm. |
/// | `m_status` | Primitive u32 | 0 = Draft, 1 = Issued. |
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Revision {
    pub sequence_number: Option<u32>,
    pub revision_date: Option<String>,
    pub description: Option<String>,
    pub issued_to: Option<String>,
    pub issued_by: Option<String>,
    pub status: Option<RevisionStatus>,
}

impl Revision {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                (
                    "sequencenumber" | "sequence" | "number",
                    InstanceField::Integer { value, .. },
                ) => {
                    out.sequence_number = Some(*value as u32);
                }
                ("revisiondate" | "date", InstanceField::String(s)) => {
                    out.revision_date = Some(s.clone());
                }
                ("description", InstanceField::String(s)) => {
                    out.description = Some(s.clone());
                }
                ("issuedto", InstanceField::String(s)) => {
                    out.issued_to = Some(s.clone());
                }
                ("issuedby", InstanceField::String(s)) => {
                    out.issued_by = Some(s.clone());
                }
                ("status" | "revisionstatus", InstanceField::Integer { value, .. }) => {
                    out.status = Some(RevisionStatus::from_code(*value as u32));
                }
                _ => {}
            }
        }
        out
    }

    /// Is this revision finalised (`Issued` status)?
    pub fn is_issued(&self) -> bool {
        matches!(self.status, Some(RevisionStatus::Issued))
    }
}
