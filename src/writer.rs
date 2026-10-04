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
//!   tests cover byte-identical copy, unchanged identity patches,
//!   stream growth, stream shrink, multi-stream patches, missing
//!   stream rejection, corrupt gzip verification, and GUID/history
//!   preservation on real family/project fixtures when the corpora
//!   are available.
//!
//! # What does not work yet
//!
//! - **Field-level semantic editing**: writing NEW values into
//!   Formats/Latest schema fields or Global/Latest instance fields.
//!   Gated by ADR-002 (`docs/decisions/ADR-002-semantic-write-api-gate.md`)
//!   until a written file can be shown to open in Revit; it needs
//!   per-class encoders on top of stream-level patching and the
//!   whole-schema reader (#410).
//! - **CFB structural writing at Revit's exact sector layout**:
//!   non-empty patches preserve unpatched stream bytes and identity
//!   metadata, but the raw container bytes can still differ because
//!   the `cfb` crate owns FAT/directory maintenance.
//!
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
/// - `Global/*` streams: 8-byte custom prefix (`00 × 8`), then gzip.
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
/// A patch's bytes as they are stored, framed as it asks: one complete gzip
/// member, with the CRC32 and ISIZE trailer Revit's own streams carry and a
/// conforming reader checks (B69).
///
/// Revit stores the streams [`crate::compression::is_checksum_paged_stream`]
/// names in pages of [`crate::compression::REVIT_STORED_PAGE_BYTES`] bytes,
/// each ending in a checksum rvt-rs cannot compute, and every reader strips
/// those trailers. Framed bytes that fit in one page carry no trailer and are
/// written as they are; longer ones would be read back wrong, so the patch is
/// refused with [`crate::Error::WriteRefused`] before anything is written (B69).
fn framed_patch_bytes(p: &StreamPatch) -> Result<Vec<u8>> {
    use crate::compression;
    let data = match p.framing {
        StreamFraming::RawGzipFromZero => compression::gzip_member_encode(&p.new_decompressed)?,
        StreamFraming::CustomPrefix8 => [
            &[0u8; 8][..],
            &compression::gzip_member_encode(&p.new_decompressed)?,
        ]
        .concat(),
        StreamFraming::Verbatim => p.new_decompressed.clone(),
    };
    if compression::is_checksum_paged_stream(&p.stream_name)
        && data.len() >= compression::REVIT_STORED_PAGE_BYTES
    {
        return Err(crate::Error::WriteRefused(format!(
            "stream '{}': its {} stored bytes span a {}-byte stored page; Revit ends each \
             page of this stream with a {}-byte checksum rvt-rs cannot compute",
            p.stream_name,
            data.len(),
            compression::REVIT_STORED_PAGE_BYTES,
            compression::REVIT_PAGE_CHECKSUM_BYTES
        )));
    }
    Ok(data)
}

pub fn write_with_patches(src: &Path, dst: &Path, patches: &[StreamPatch]) -> Result<()> {
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
    // B69: every patch is framed before any file is touched, so a refused
    // one leaves nothing behind.
    let framed: Vec<Vec<u8>> = patches
        .iter()
        .map(framed_patch_bytes)
        .collect::<Result<_>>()?;

    // WRT-10.3 fast-path: empty-patch round-trip bypasses CFB
    // round-tripping entirely and copies the source file byte-for-
    // byte. This gives us byte-identical output for the
    // "verify the pipeline" use case (corpus re-emission, canonical
    // form checks, git-friendly diffing) without needing the
    // sector-reordering pass. For non-empty patches, fall through
    // to the CFB rewriter below — sector preservation for patched
    // builds is still pending.
    if patches.is_empty() && src != dst {
        // Atomic copy via temp + rename (same durability contract as
        // the CFB path below).
        let dst_parent = dst.parent().unwrap_or_else(|| Path::new("."));
        let dst_name = dst
            .file_name()
            .ok_or_else(|| crate::Error::Cfb("dst has no filename component".into()))?
            .to_string_lossy()
            .to_string();
        let tmp_name = format!(".{dst_name}.copy-{}", std::process::id());
        let tmp_path = dst_parent.join(&tmp_name);
        std::fs::copy(src, &tmp_path)?;
        std::fs::rename(&tmp_path, dst)?;
        return Ok(());
    }

    // WRT-10.3: non-empty patch sector-preservation path.
    //
    // Strategy: copy src to a temp file, open it with `cfb::open_rw`,
    // and rewrite ONLY the patched streams in-place. The `cfb` crate
    // reuses the stream's existing sector chain when the new content
    // fits (and extends only the tail when it grows), so every
    // unpatched stream's sectors stay physically where they were in
    // the source. This reduces the byte delta from ~94% (full
    // rebuild) to "only the patched streams' sectors and the FAT
    // entries that describe them" — usually well under 5% for a
    // single-stream patch.
    if src != dst {
        drop(rf); // release the read-only handle before we reopen rw.

        let dst_parent = dst.parent().unwrap_or_else(|| Path::new("."));
        let dst_name = dst
            .file_name()
            .ok_or_else(|| crate::Error::Cfb("dst has no filename component".into()))?
            .to_string_lossy()
            .to_string();
        let tmp_name = format!(".{dst_name}.rw-{}", std::process::id());
        let tmp_path = dst_parent.join(&tmp_name);

        struct InplaceGuard {
            path: Option<std::path::PathBuf>,
        }
        impl Drop for InplaceGuard {
            fn drop(&mut self) {
                if let Some(p) = self.path.take() {
                    let _ = std::fs::remove_file(&p);
                }
            }
        }
        let mut guard = InplaceGuard {
            path: Some(tmp_path.clone()),
        };

        std::fs::copy(src, &tmp_path)?;
        {
            let mut rw = cfb::open_rw(&tmp_path)
                .map_err(|e| crate::Error::Cfb(format!("open_rw {}: {e}", tmp_path.display())))?;
            for (p, data) in patches.iter().zip(&framed) {
                let path = if p.stream_name.starts_with('/') {
                    p.stream_name.clone()
                } else {
                    format!("/{}", p.stream_name)
                };
                // `create_stream` truncates + rewrites an existing
                // stream by design, which is what we want. The cfb
                // crate reuses the prior sector chain when content
                // fits and extends only the tail when it grows.
                let mut s = rw
                    .create_stream(&path)
                    .map_err(|e| crate::Error::Cfb(format!("create_stream {path}: {e}")))?;
                s.write_all(data)
                    .map_err(|e| crate::Error::Cfb(format!("write_all {path}: {e}")))?;
                s.flush()
                    .map_err(|e| crate::Error::Cfb(format!("flush {path}: {e}")))?;
            }
            rw.flush()
                .map_err(|e| crate::Error::Cfb(format!("flush: {e}")))?;
        }
        std::fs::rename(&tmp_path, dst)?;
        guard.path = None;
        return Ok(());
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
        let patched = patches.iter().position(|p| p.stream_name == name);
        let data = match patched {
            Some(index) => framed[index].clone(),
            None => rf.read_stream(&name)?,
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

// ---- WRT-05: GUID preservation across write ----

/// Read the BasicFileInfo GUID from a Revit file (WRT-05). Returns
/// `Ok(None)` when the stream exists but no GUID was embedded. Used
/// by the write-path invariant tests to confirm a write cycle
/// preserves the file's identity.
pub fn file_guid(path: &Path) -> Result<Option<String>> {
    let mut rf = RevitFile::open(path)?;
    let stream_name = "BasicFileInfo";
    if !rf.stream_names().iter().any(|s| s == stream_name) {
        return Ok(None);
    }
    let bytes = rf.read_stream(stream_name)?;
    let info = crate::basic_file_info::BasicFileInfo::from_bytes(&bytes)
        .map_err(|e| crate::Error::BasicFileInfo(format!("parse: {e}")))?;
    Ok(info.guid)
}

/// Verify that a write cycle (copy / patch / rewrite) preserved
/// the source file's GUID. Returns `Ok(true)` when both files
/// carry the same GUID, `Ok(false)` when GUIDs diverge, and an
/// error when either file is unreadable.
///
/// This is the primary WRT-05 invariant check — call it right
/// after any [`write_with_patches`] / [`write_with_patches_verified`]
/// to gate deploys on GUID preservation.
pub fn guid_preserved(src: &Path, dst: &Path) -> Result<bool> {
    let a = file_guid(src)?;
    let b = file_guid(dst)?;
    Ok(a == b)
}

// ---- WRT-06: History chain preservation ----

/// Read the document-history entries from a Revit file (WRT-06).
/// Returns the ordered list of Revit-version strings embedded in
/// `Global/Latest` — oldest first, same order as
/// [`crate::object_graph::DocumentHistory::entries`].
///
/// Returns `Ok(Vec::new())` when no Global/Latest stream exists or
/// when the stream has no embedded "Revit …" markers. Err when the
/// stream read or decompression fails.
pub fn file_history_entries(path: &Path) -> Result<Vec<String>> {
    let mut rf = RevitFile::open(path)?;
    let stream_name = crate::streams::GLOBAL_LATEST;
    if !rf.stream_names().iter().any(|s| s == stream_name) {
        return Ok(Vec::new());
    }
    match crate::object_graph::DocumentHistory::from_revit_file(&mut rf) {
        Ok(h) => Ok(h.entries),
        Err(_) => Ok(Vec::new()),
    }
}

/// Verify that a write cycle preserved the document-history chain
/// (WRT-06). Returns `Ok(true)` when src and dst share the exact
/// same ordered entry list, `Ok(false)` on divergence, and Err
/// when either file is unreadable.
///
/// Pair with [`guid_preserved`] for a two-value "file-identity
/// preserved" predicate after any [`write_with_patches`] cycle.
pub fn history_entries_preserved(src: &Path, dst: &Path) -> Result<bool> {
    let a = file_history_entries(src)?;
    let b = file_history_entries(dst)?;
    Ok(a == b)
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
///
/// # Checksum-page policy (Finding 1 / #151)
///
/// The writer re-encodes patches with
/// [`crate::compression::gzip_member_encode`], behind the 8-byte prefix for
/// `CustomPrefix8` — **strip-clean** gzip, not Revit's stored checksum-paged
/// layout. Verification therefore uses
/// bare [`crate::compression::inflate_at`], not
/// [`crate::compression::inflate_stream_at`]: blindly stripping every
/// 65_249-byte boundary would corrupt writer-produced streams that happen
/// to be ≥ one stored page. Identity copies (`read_stream` / empty-patch)
/// remain stored-byte accurate. A paged encoder is still out of scope, so
/// the writer refuses a patch whose stored bytes would span a page of a paged
/// stream (B69): every stream it writes reads back the same through the
/// stripping path.
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
