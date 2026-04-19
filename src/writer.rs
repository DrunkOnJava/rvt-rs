//! Write path — round-trip Revit files.
//!
//! # Current scope (Phase-gated scaffold)
//!
//! This module is the Phase 6 entry point: reading a Revit file and
//! writing it back out **byte-for-byte identically**. Once that round-trip
//! works, modifying a stream's content becomes a matter of:
//!
//! 1. Decompress stream → structured data (Layer 4c).
//! 2. Edit the structured data.
//! 3. Serialize back to bytes using the inverse of Layer 4c.
//! 4. Re-compress with truncated gzip and re-embed into the OLE container.
//!
//! Step 4 is what this module currently addresses. Steps 1–3 are gated
//! on Layer 4c (object-graph field decoding) completing; the write path
//! for the object-graph bytes is therefore not yet implemented.
//!
//! # What works today
//!
//! - Copy a Revit file from one path to another by re-reading every OLE
//!   stream and re-writing it through a new `cfb::CompoundFile`. This is
//!   mostly a smoke test for the container-level round trip.
//!
//! # What does not work yet
//!
//! - Modifying an OLE stream's content. The stream-encoding invariants
//!   (truncated-gzip framing, custom 8-byte prefix on Global/*, Revit
//!   wrapper on RevitPreview4.0 and Contents) must be recreated byte-for-
//!   byte. Tests ensure we preserve them on copy but do not yet verify
//!   arbitrary edits.

use crate::{Result, RevitFile};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

/// Copy a Revit file from `src` to `dst`. The output container is a fresh
/// OLE2 CFB written by the `cfb` crate; stream contents are copied
/// byte-for-byte. For equivalence, check byte-level equality of the
/// decompressed streams on both sides — not the raw file bytes, because
/// OLE sector ordering may differ.
pub fn copy_file(src: &Path, dst: &Path) -> Result<()> {
    let mut rf = RevitFile::open(src)?;
    let streams = rf.stream_names();
    let out_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(dst)?;
    let mut out = cfb::CompoundFile::create(out_file)
        .map_err(|e| crate::Error::Cfb(format!("create dst: {e}")))?;

    // Create parent storages first. OLE2 requires `/Formats`, `/Global`,
    // `/Partitions` to exist as storages before their child streams can
    // be created. Walk the stream list and pre-create every intermediate
    // folder.
    let mut created: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for name in &streams {
        let norm = if name.starts_with('/') {
            name.clone()
        } else {
            format!("/{name}")
        };
        let parts: Vec<&str> = norm.split('/').filter(|s| !s.is_empty()).collect();
        for n in 1..parts.len() {
            let parent = format!("/{}", parts[..n].join("/"));
            if created.insert(parent.clone()) {
                out.create_storage(&parent).map_err(|e| {
                    crate::Error::Cfb(format!("create_storage {parent}: {e}"))
                })?;
            }
        }
    }

    for name in streams {
        let data = rf.read_stream(&name)?;
        let path = if name.starts_with('/') {
            name.clone()
        } else {
            format!("/{name}")
        };
        let mut s = out
            .create_stream(&path)
            .map_err(|e| crate::Error::Cfb(format!("create_stream {path}: {e}")))?;
        s.write_all(&data)
            .map_err(|e| crate::Error::Cfb(format!("write_all {path}: {e}")))?;
    }
    out.flush()
        .map_err(|e| crate::Error::Cfb(format!("flush: {e}")))?;
    Ok(())
}

/// Marker error indicating that a modifying write-path is not yet implemented
/// because the Layer 4c round-trip isn't available.
#[derive(Debug, Clone, thiserror::Error)]
#[error("write path not implemented: Layer 4c field encoding is incomplete")]
pub struct NotYetImplemented;

/// Placeholder for "modify this class instance and write the file back".
/// Currently always returns `Err(NotYetImplemented)`. The signature is
/// stable enough that downstream tools can depend on it.
pub fn write_with_patch<P: AsRef<Path>>(
    _src: P,
    _dst: P,
    _patches: &[()],
) -> std::result::Result<(), NotYetImplemented> {
    Err(NotYetImplemented)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_yet_implemented_returns_expected_error() {
        let r = write_with_patch::<&str>("a", "b", &[]);
        assert!(r.is_err());
    }
}
