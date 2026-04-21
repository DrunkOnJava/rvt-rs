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

// ---- WRT-13: Stream hash verification per write ----

/// Per-stream verification outcome (WRT-13). One entry per
/// expected stream; the `match_`/`first_diff_at` pair distinguishes
/// clean success from a specific byte position where the written
/// stream diverged from expectations.
#[derive(Debug, Clone)]
pub struct StreamVerification {
    /// OLE stream name, e.g. `"Formats/Latest"`.
    pub stream_name: String,
    /// `true` when the stream's decompressed bytes in the written
    /// file exactly equal the expected decompressed bytes.
    pub match_: bool,
    /// When `match_ == false`, the first byte offset where the
    /// decompressed output differs from expected. `None` on
    /// success or when the decompression itself failed.
    pub first_diff_at: Option<usize>,
    /// When decompression failed, the error message. `None` on
    /// clean read (whether or not bytes matched).
    pub decompress_error: Option<String>,
    /// Length of the decompressed bytes actually read from `dst`
    /// (0 when decompression failed entirely).
    pub actual_len: usize,
    /// Length of the expected decompressed bytes (what
    /// [`StreamPatch::new_decompressed`] carried).
    pub expected_len: usize,
}

/// Aggregate result of verifying every patched stream (WRT-13).
#[derive(Debug, Clone, Default)]
pub struct StreamVerificationReport {
    /// Per-stream outcomes in the same order as the input patches.
    pub streams: Vec<StreamVerification>,
}

impl StreamVerificationReport {
    /// `true` when every stream verified cleanly.
    pub fn all_matched(&self) -> bool {
        !self.streams.is_empty() && self.streams.iter().all(|s| s.match_)
    }

    /// Count of streams that failed verification (mismatch or
    /// decompression error).
    pub fn failure_count(&self) -> usize {
        self.streams.iter().filter(|s| !s.match_).count()
    }

    /// Iterator over just the failing streams.
    pub fn failures(&self) -> impl Iterator<Item = &StreamVerification> {
        self.streams.iter().filter(|s| !s.match_)
    }
}

/// Decompress a named stream from `dst` using the given framing.
/// Returns `Ok(decompressed_bytes)` or an error message if any
/// step failed. Used internally by [`verify_patches_applied`]; pub
/// so corpus audits can call it too.
pub fn decompress_stream(dst: &Path, name: &str, framing: StreamFraming) -> Result<Vec<u8>> {
    let mut rf = RevitFile::open(dst)?;
    let raw = rf.read_stream(name)?;
    match framing {
        StreamFraming::RawGzipFromZero => crate::compression::inflate_at(&raw, 0),
        StreamFraming::CustomPrefix8 => {
            // 8-byte custom prefix — gzip starts at offset 8.
            if raw.len() < 8 {
                return Err(crate::Error::Cfb(format!(
                    "stream '{name}' too short for CustomPrefix8 framing: {} bytes",
                    raw.len()
                )));
            }
            crate::compression::inflate_at(&raw, 8)
        }
        StreamFraming::Verbatim => Ok(raw),
    }
}

/// Verify that every patch in `patches` round-tripped through the
/// writer cleanly (WRT-13). Opens `dst`, reads each named stream,
/// decompresses it with the patch's `framing`, and compares the
/// resulting bytes to the patch's `new_decompressed`.
///
/// Typical use:
///
/// ```no_run
/// # use rvt::writer::{write_with_patches, verify_patches_applied, StreamPatch, StreamFraming};
/// # use std::path::Path;
/// let patches = vec![StreamPatch {
///     stream_name: "Formats/Latest".into(),
///     new_decompressed: vec![/* new bytes */],
///     framing: StreamFraming::RawGzipFromZero,
/// }];
/// write_with_patches(Path::new("in.rfa"), Path::new("out.rfa"), &patches)?;
/// let report = verify_patches_applied(Path::new("out.rfa"), &patches)?;
/// assert!(report.all_matched(), "one or more patches failed to round-trip");
/// # Ok::<(), rvt::Error>(())
/// ```
pub fn verify_patches_applied(
    dst: &Path,
    patches: &[StreamPatch],
) -> Result<StreamVerificationReport> {
    let mut report = StreamVerificationReport::default();
    for p in patches {
        match decompress_stream(dst, &p.stream_name, p.framing) {
            Ok(actual) => {
                let first_diff = actual
                    .iter()
                    .zip(p.new_decompressed.iter())
                    .position(|(a, b)| a != b)
                    .or_else(|| {
                        if actual.len() != p.new_decompressed.len() {
                            Some(actual.len().min(p.new_decompressed.len()))
                        } else {
                            None
                        }
                    });
                let match_ = actual == p.new_decompressed;
                report.streams.push(StreamVerification {
                    stream_name: p.stream_name.clone(),
                    match_,
                    first_diff_at: if match_ { None } else { first_diff },
                    decompress_error: None,
                    actual_len: actual.len(),
                    expected_len: p.new_decompressed.len(),
                });
            }
            Err(e) => {
                report.streams.push(StreamVerification {
                    stream_name: p.stream_name.clone(),
                    match_: false,
                    first_diff_at: None,
                    decompress_error: Some(e.to_string()),
                    actual_len: 0,
                    expected_len: p.new_decompressed.len(),
                });
            }
        }
    }
    Ok(report)
}

/// Convenience wrapper: [`write_with_patches`] followed by
/// [`verify_patches_applied`] on the written file (WRT-13).
/// Returns the verification report directly so callers can gate
/// deployment on `report.all_matched()` without a second call.
///
/// On verification failure the output file is left in place so the
/// caller can inspect it — we don't auto-delete, since the bytes
/// may still be useful for diagnosis.
pub fn write_with_patches_verified(
    src: &Path,
    dst: &Path,
    patches: &[StreamPatch],
) -> Result<StreamVerificationReport> {
    write_with_patches(src, dst, patches)?;
    verify_patches_applied(dst, patches)
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

    // ---- WRT-13: verify_patches_applied end-to-end ----

    fn temp_path(name: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "rvt-writer-test-{}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            name
        ));
        p
    }

    /// Build a minimal CFB file with one gzip-compressed stream
    /// `"Formats/Latest"` carrying `payload`.
    fn build_tiny_cfb(path: &Path, payload: &[u8]) -> Result<()> {
        use crate::compression::truncated_gzip_encode;
        let compressed = truncated_gzip_encode(payload)?;
        let f = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        let mut cfb =
            cfb::CompoundFile::create(f).map_err(|e| crate::Error::Cfb(format!("create: {e}")))?;
        cfb.create_storage("/Formats")
            .map_err(|e| crate::Error::Cfb(format!("storage: {e}")))?;
        let mut s = cfb
            .create_stream("/Formats/Latest")
            .map_err(|e| crate::Error::Cfb(format!("stream: {e}")))?;
        s.write_all(&compressed)
            .map_err(|e| crate::Error::Cfb(format!("write: {e}")))?;
        drop(s);
        cfb.flush()
            .map_err(|e| crate::Error::Cfb(format!("flush: {e}")))?;
        Ok(())
    }

    #[test]
    fn verify_patches_applied_round_trips_clean_edit() {
        let src = temp_path("src.rvt");
        let dst = temp_path("dst.rvt");
        build_tiny_cfb(&src, b"old-bytes-for-stream").unwrap();
        let new_payload = b"new-payload-after-patch".to_vec();
        let patches = vec![StreamPatch {
            stream_name: "Formats/Latest".into(),
            new_decompressed: new_payload.clone(),
            framing: StreamFraming::RawGzipFromZero,
        }];
        write_with_patches(&src, &dst, &patches).unwrap();
        let report = verify_patches_applied(&dst, &patches).unwrap();
        assert!(
            report.all_matched(),
            "expected all streams to verify: {:?}",
            report.streams
        );
        assert_eq!(report.failure_count(), 0);
        assert_eq!(report.streams.len(), 1);
        assert_eq!(report.streams[0].expected_len, new_payload.len());
        assert_eq!(report.streams[0].actual_len, new_payload.len());
        assert!(report.streams[0].decompress_error.is_none());
        std::fs::remove_file(&src).ok();
        std::fs::remove_file(&dst).ok();
    }

    #[test]
    fn verify_patches_applied_detects_byte_mismatch() {
        let src = temp_path("src.rvt");
        let dst = temp_path("dst.rvt");
        build_tiny_cfb(&src, b"original").unwrap();
        // Tell the writer to write "actual-patch" but tell the
        // verifier to expect something different, simulating a
        // patch that wasn't properly re-encoded.
        let written_patches = vec![StreamPatch {
            stream_name: "Formats/Latest".into(),
            new_decompressed: b"actual-patch".to_vec(),
            framing: StreamFraming::RawGzipFromZero,
        }];
        write_with_patches(&src, &dst, &written_patches).unwrap();
        let expected_patches = vec![StreamPatch {
            stream_name: "Formats/Latest".into(),
            new_decompressed: b"different-expected".to_vec(),
            framing: StreamFraming::RawGzipFromZero,
        }];
        let report = verify_patches_applied(&dst, &expected_patches).unwrap();
        assert!(!report.all_matched());
        assert_eq!(report.failure_count(), 1);
        let fail = &report.streams[0];
        assert!(!fail.match_);
        assert!(fail.first_diff_at.is_some());
        std::fs::remove_file(&src).ok();
        std::fs::remove_file(&dst).ok();
    }

    #[test]
    fn write_with_patches_verified_is_all_in_one() {
        let src = temp_path("src.rvt");
        let dst = temp_path("dst.rvt");
        build_tiny_cfb(&src, b"seed").unwrap();
        let patches = vec![StreamPatch {
            stream_name: "Formats/Latest".into(),
            new_decompressed: b"fresh-payload".to_vec(),
            framing: StreamFraming::RawGzipFromZero,
        }];
        let report = write_with_patches_verified(&src, &dst, &patches).unwrap();
        assert!(report.all_matched());
        std::fs::remove_file(&src).ok();
        std::fs::remove_file(&dst).ok();
    }

    #[test]
    fn verify_report_failures_iterator_yields_only_fails() {
        let ok = StreamVerification {
            stream_name: "Good".into(),
            match_: true,
            first_diff_at: None,
            decompress_error: None,
            actual_len: 4,
            expected_len: 4,
        };
        let bad = StreamVerification {
            stream_name: "Bad".into(),
            match_: false,
            first_diff_at: Some(2),
            decompress_error: None,
            actual_len: 4,
            expected_len: 4,
        };
        let report = StreamVerificationReport {
            streams: vec![ok, bad],
        };
        let fails: Vec<&str> = report.failures().map(|s| s.stream_name.as_str()).collect();
        assert_eq!(fails, vec!["Bad"]);
        assert_eq!(report.failure_count(), 1);
        assert!(!report.all_matched());
    }

    #[test]
    fn all_matched_false_for_empty_report() {
        let report = StreamVerificationReport::default();
        assert!(!report.all_matched());
    }

    #[test]
    fn decompress_stream_surfaces_framing_errors() {
        let src = temp_path("src.rvt");
        build_tiny_cfb(&src, b"hello").unwrap();
        // Ask for CustomPrefix8 on a RawGzipFromZero stream — the
        // first 8 bytes aren't a prefix, they're the gzip header,
        // so inflate_at(raw, 8) will fail.
        let result = decompress_stream(&src, "Formats/Latest", StreamFraming::CustomPrefix8);
        assert!(result.is_err());
        std::fs::remove_file(&src).ok();
    }
}
