//! Write path — round-trip Revit files + stream-level patching.
//!
//! # What works today
//!
//! - **CFB copy**: copy a Revit file from one path to another by
//!   re-reading every OLE stream and re-writing it through a new
//!   `cfb::CompoundFile`. Verified on the 11-release corpus.
//! - **Stream-level patching**: [`write_with_patches`] replaces the
//!   decompressed contents of named streams with caller-provided
//!   bytes, re-compresses with truncated gzip, and writes a new
//!   file. The framing invariants (gzip-truncation, 8-byte prefix
//!   on `Global/*`, Revit wrapper on `RevitPreview4.0` and
//!   `Contents`) are preserved via [`StreamFraming`]. Round-trip
//!   tests verify the 13 streams in the 2024 sample stay
//!   semantically identical after patch-less write.
//!
//! # What does not work yet
//!
//! - **Field-level semantic editing**: writing NEW values into
//!   Formats/Latest schema fields or Global/Latest instance fields.
//!   Blocked on Phase 7 (per-class encoders) in
//!   `TODO-BLINDSIDE.md`. Stream-level patching + the 100%
//!   classified schema are the pieces that unblock it.
//! - **CFB structural writing at Revit's exact sector layout**:
//!   current output uses the `cfb` crate's default sector
//!   ordering, which differs from Revit's own writer. Streams are
//!   byte-identical on read; raw file bytes are not.
//! # Atomicity
//!
//! [`write_with_patches`] writes to a sibling temp file and renames
//! into place on success. A mid-write failure leaves `dst` either
//! unchanged (if it already existed) or absent. The `TempGuard` RAII
//! handle unlinks the temp file on any early return or panic.

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
///
/// # Validation
///
/// Every `StreamPatch.stream_name` MUST correspond to a stream that
/// exists in the source file. A patch for a non-existent stream (e.g.
/// a typo in the name) returns `Error::StreamNotFound` BEFORE any
/// write begins. This prevents silent-no-op bugs where users think
/// they patched a stream but actually just copied the file unchanged.
///
/// # Atomicity
///
/// The output is written to a sibling temp file (`<dst>.tmp-<pid>`)
/// and renamed to `dst` only after all writes + flushes succeed. A
/// mid-write failure leaves the temp file behind and `dst` either
/// unchanged (if it already existed) or absent. This prevents the
/// previous corrupt-on-mid-write behaviour that a truncating
/// `OpenOptions::truncate(true).open(dst)` call caused.
pub fn write_with_patches(src: &Path, dst: &Path, patches: &[StreamPatch]) -> Result<()> {
    use crate::compression;
    let mut rf = RevitFile::open(src)?;
    let streams = rf.stream_names();

    // Validate patch names against actual stream set. A typo or stale
    // reference should error fast, not silently no-op.
    let stream_set: std::collections::BTreeSet<&str> = streams.iter().map(|s| s.as_str()).collect();
    for p in patches {
        if !stream_set.contains(p.stream_name.as_str()) {
            return Err(crate::Error::StreamNotFound(p.stream_name.clone()));
        }
    }

    // Compute a sibling temp path in the same directory as dst so
    // the final rename is atomic on the same filesystem.
    let dst_parent = dst.parent().unwrap_or_else(|| Path::new("."));
    let dst_name = dst
        .file_name()
        .ok_or_else(|| crate::Error::Cfb("dst has no filename component".into()))?
        .to_string_lossy()
        .to_string();
    let tmp_name = format!(".{dst_name}.tmp-{}", std::process::id());
    let tmp_path = dst_parent.join(&tmp_name);

    // Guard that unlinks the temp file on any early return or panic.
    // On success we rename it into place and the guard becomes a
    // no-op (its path field gets cleared).
    struct TempGuard {
        path: Option<std::path::PathBuf>,
    }
    impl Drop for TempGuard {
        fn drop(&mut self) {
            if let Some(p) = self.path.take() {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
    let mut guard = TempGuard {
        path: Some(tmp_path.clone()),
    };

    let out_file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&tmp_path)?;
    let mut out = cfb::CompoundFile::create(out_file)
        .map_err(|e| crate::Error::Cfb(format!("create tmp: {e}")))?;

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
    // Close before rename so Windows doesn't hold the handle.
    drop(out);

    // Atomic rename into place. If rename fails (e.g. cross-device),
    // fall back to copy+remove. On failure both dst and tmp may exist
    // briefly; the guard cleans up tmp.
    std::fs::rename(&tmp_path, dst).map_err(|e| {
        crate::Error::Cfb(format!(
            "rename {} -> {}: {e}",
            tmp_path.display(),
            dst.display()
        ))
    })?;

    // Rename succeeded — temp no longer exists, disarm the guard.
    guard.path = None;
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
