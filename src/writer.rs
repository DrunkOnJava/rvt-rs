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
                out.create_storage(&parent)
                    .map_err(|e| crate::Error::Cfb(format!("create_storage {parent}: {e}")))?;
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

/// A stream-level patch: replace the decompressed payload of a named OLE
/// stream with new bytes. The writer handles re-compression + re-embedding
/// into the OLE container.
#[derive(Debug, Clone)]
pub struct StreamPatch {
    /// OLE stream name, e.g. `"Formats/Latest"` or `"Global/Latest"`.
    pub stream_name: String,
    /// New decompressed payload.
    pub new_decompressed: Vec<u8>,
    /// Framing to use when re-encoding. See [`StreamFraming`].
    pub framing: StreamFraming,
}

/// How a stream's compressed body should be framed on disk. Revit uses
/// two distinct conventions:
///
/// - `Global/*` streams: 8-byte custom prefix (`00 × 8`), then truncated gzip.
/// - `Formats/Latest` and `Global/ContentDocuments`: gzip from byte 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamFraming {
    /// Gzip starts at offset 0 (e.g. Formats/Latest).
    RawGzipFromZero,
    /// 8-byte custom prefix + gzip (e.g. Global/Latest).
    CustomPrefix8,
    /// Stream has a completely custom wrapper + embedded gzip, or is
    /// uncompressed. Bytes are written verbatim — caller is responsible
    /// for framing.
    Verbatim,
}

/// Write `src` to `dst`, applying `patches` (by stream name) along the way.
/// Streams not mentioned in `patches` are copied byte-for-byte.
///
/// Success criterion: the round-trip preserves every unpatched stream
/// byte-for-byte; patched streams round-trip with their new content.
pub fn write_with_patches(src: &Path, dst: &Path, patches: &[StreamPatch]) -> Result<()> {
    use crate::compression;
    let mut rf = RevitFile::open(src)?;
    let streams = rf.stream_names();
    let out_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(dst)?;
    let mut out = cfb::CompoundFile::create(out_file)
        .map_err(|e| crate::Error::Cfb(format!("create dst: {e}")))?;

    // Pre-create parent storages (same logic as copy_file).
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
                out.create_storage(&parent)
                    .map_err(|e| crate::Error::Cfb(format!("create_storage {parent}: {e}")))?;
            }
        }
    }

    for name in streams {
        let patch = patches.iter().find(|p| p.stream_name == name);
        let data = if let Some(p) = patch {
            match p.framing {
                StreamFraming::RawGzipFromZero => {
                    compression::truncated_gzip_encode(&p.new_decompressed)?
                }
                StreamFraming::CustomPrefix8 => {
                    compression::truncated_gzip_encode_with_prefix8(&p.new_decompressed)?
                }
                StreamFraming::Verbatim => p.new_decompressed.clone(),
            }
        } else {
            rf.read_stream(&name)?
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_patch_struct_builds() {
        let p = StreamPatch {
            stream_name: "Formats/Latest".into(),
            new_decompressed: vec![0x1fu8, 0x8b, 0x08],
            framing: StreamFraming::RawGzipFromZero,
        };
        assert_eq!(p.stream_name, "Formats/Latest");
        assert_eq!(p.framing, StreamFraming::RawGzipFromZero);
    }
}
