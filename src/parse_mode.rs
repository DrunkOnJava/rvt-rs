//! Strict vs lossy parse modes + diagnostics.
//!
//! Audit P1 identified the single most important API discipline
//! problem in rvt-rs: the same method (e.g. `RevitFile::summarize`)
//! sometimes returned partial results with errors silently swallowed,
//! and sometimes returned clean results — with no way for callers to
//! tell which. That's fine for a CLI that wants to show whatever it
//! can; it's dangerous for a library that might be wrapped in a SaaS
//! file-validation pipeline where partial parse = false confidence.
//!
//! This module introduces the three types that every future
//! strict/lossy pair uses:
//!
//! - [`ParseMode`] — the intent switch. Strict errors on any parse
//!   failure; BestEffort collects diagnostics and returns partial.
//! - [`Diagnostics`] — warnings + failure records accumulated during
//!   a best-effort parse.
//! - [`Decoded<T>`] — wrapper returned by lossy variants. Pairs the
//!   value with its diagnostics + a `complete` flag.
//!
//! New methods that follow this pattern are named `*_strict` and
//! `*_lossy` to make the choice obvious at the call site. Existing
//! methods that previously had mixed behaviour are aliased to
//! `_lossy` via `#[deprecated]` until a later major version cleans
//! them up.

use serde::{Deserialize, Serialize};

/// Parser-intent switch for operations that can choose between
/// strict and best-effort behaviour.
///
/// Most callers don't touch this directly — they pick `*_strict` or
/// `*_lossy` method variants. The enum exists for cases where the
/// choice is data-driven (a higher-level pipeline might decide strict
/// for "inbound upload validation" and best-effort for "archival
/// metadata extraction").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ParseMode {
    /// Any parse failure returns Err. Useful for SaaS pipelines
    /// where "looks like a valid Revit file but isn't" should be
    /// flagged explicitly rather than silently missing data.
    Strict,
    /// Collect diagnostics for every parse failure, continue where
    /// possible, return partial results. Useful for forensic /
    /// CLI workflows where seeing what's there matters more than
    /// integrity.
    #[default]
    BestEffort,
}

/// A single warning accumulated during a best-effort parse.
///
/// Every `Diagnostics` entry is a structured record — no
/// free-form strings — so downstream tooling can filter and count
/// them without pattern-matching on prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Warning {
    /// Short machine-readable code identifying the class of warning
    /// (e.g. `"schema-parse-skipped"`, `"stream-inflate-failed"`,
    /// `"field-type-unknown"`).
    pub code: String,
    /// Human-readable detail — includes specifics like stream name,
    /// byte offset, field name. Safe to log.
    pub message: String,
    /// Optional byte offset where the issue was observed. Useful
    /// when the warning relates to a specific record in a stream.
    pub offset: Option<usize>,
}

impl Warning {
    /// Construct a Warning with a code + message and no offset.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            offset: None,
        }
    }

    /// Construct a Warning with a code + message + byte offset.
    pub fn at(code: impl Into<String>, message: impl Into<String>, offset: usize) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            offset: Some(offset),
        }
    }
}

/// Diagnostic accumulator carried by best-effort parse results.
///
/// Callers who want to verify that a lossy parse actually produced
/// a complete result should check `.is_empty()` or inspect specific
/// warnings / partial-field lists.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Diagnostics {
    /// Warnings encountered during the parse.
    pub warnings: Vec<Warning>,
    /// Number of records the parser had to skip (e.g. schema records
    /// that didn't validate their checksum, streams that failed to
    /// inflate, etc.).
    pub skipped_records: usize,
    /// Stream names that failed to open or read.
    pub failed_streams: Vec<String>,
    /// Field names that produced partial or fallback values (e.g.
    /// InstanceField::Bytes where a structured decode was expected).
    pub partial_fields: Vec<String>,
    /// Optional confidence score, 0.0–1.0. Currently only populated
    /// by the walker's entry-point detector; other sources may
    /// ignore it.
    pub confidence: Option<f32>,
}

impl Diagnostics {
    /// True when no warnings, no skipped records, no failures.
    /// Implies the underlying value is complete.
    pub fn is_empty(&self) -> bool {
        self.warnings.is_empty()
            && self.skipped_records == 0
            && self.failed_streams.is_empty()
            && self.partial_fields.is_empty()
    }

    /// Record a warning.
    pub fn warn(&mut self, w: Warning) {
        self.warnings.push(w);
    }

    /// Record a failed stream read.
    pub fn fail_stream(&mut self, name: impl Into<String>) {
        self.failed_streams.push(name.into());
    }

    /// Record a partial field decode.
    pub fn partial_field(&mut self, name: impl Into<String>) {
        self.partial_fields.push(name.into());
    }

    /// Merge another Diagnostics into this one (additive).
    pub fn extend(&mut self, other: Diagnostics) {
        self.warnings.extend(other.warnings);
        self.skipped_records += other.skipped_records;
        self.failed_streams.extend(other.failed_streams);
        self.partial_fields.extend(other.partial_fields);
        if self.confidence.is_none() {
            self.confidence = other.confidence;
        }
    }
}

impl std::fmt::Display for Diagnostics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_empty() {
            return write!(f, "(no diagnostics)");
        }
        writeln!(f, "Diagnostics:")?;
        if !self.warnings.is_empty() {
            writeln!(f, "  Warnings ({}):", self.warnings.len())?;
            for w in &self.warnings {
                match w.offset {
                    Some(o) => writeln!(f, "    [{}] {} (@ 0x{:x})", w.code, w.message, o)?,
                    None => writeln!(f, "    [{}] {}", w.code, w.message)?,
                }
            }
        }
        if self.skipped_records > 0 {
            writeln!(f, "  Skipped records: {}", self.skipped_records)?;
        }
        if !self.failed_streams.is_empty() {
            writeln!(f, "  Failed streams:")?;
            for s in &self.failed_streams {
                writeln!(f, "    - {s}")?;
            }
        }
        if !self.partial_fields.is_empty() {
            writeln!(f, "  Partial fields:")?;
            for fld in &self.partial_fields {
                writeln!(f, "    - {fld}")?;
            }
        }
        if let Some(c) = self.confidence {
            writeln!(f, "  Confidence: {c:.2}")?;
        }
        Ok(())
    }
}

/// Wrapper returned by `*_lossy` parsers.
///
/// Pairs the parsed value with its diagnostics + a `complete` flag.
/// Callers treat the inner `value` as best-effort unless
/// `diagnostics.is_empty() && complete` — that combination is
/// equivalent to what a `*_strict` variant would have returned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Decoded<T> {
    pub value: T,
    pub diagnostics: Diagnostics,
    /// Explicit "the parser reached the end of input without
    /// giving up early" marker. False if the parser returned
    /// partial data after a failure it decided to swallow.
    pub complete: bool,
}

impl<T> Decoded<T> {
    /// Construct a Decoded from a complete parse (no diagnostics,
    /// complete=true). Shortcut for the happy path.
    pub fn complete(value: T) -> Self {
        Self {
            value,
            diagnostics: Diagnostics::default(),
            complete: true,
        }
    }

    /// Construct a Decoded from a partial parse.
    pub fn partial(value: T, diagnostics: Diagnostics) -> Self {
        Self {
            value,
            diagnostics,
            complete: false,
        }
    }

    /// Map the wrapped value through a closure, preserving
    /// diagnostics + complete flag. Functor-style convenience.
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Decoded<U> {
        Decoded {
            value: f(self.value),
            diagnostics: self.diagnostics,
            complete: self.complete,
        }
    }

    /// True iff complete + no diagnostics. Matches what a
    /// `*_strict` variant would accept.
    pub fn is_clean(&self) -> bool {
        self.complete && self.diagnostics.is_empty()
    }
}
