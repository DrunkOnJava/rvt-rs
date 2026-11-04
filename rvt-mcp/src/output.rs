//! Where an export goes and how it is written.
//!
//! An export is the one thing the server writes. It goes to the path the
//! caller names, which must carry the format's extension, must not be the
//! model that was read, and must not exist unless the caller passes
//! `overwrite`. It is written beside its target and renamed when whole, so a
//! failed or cancelled export never leaves a truncated file under the target's
//! name.

use std::fs::File;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::config::Config;
use crate::store::lower_hex;

/// A writer that counts and hashes what passes through it.
pub struct HashingWriter<W: Write> {
    inner: W,
    hasher: Sha256,
    bytes: u64,
}

impl<W: Write> HashingWriter<W> {
    fn new(inner: W) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            bytes: 0,
        }
    }

    fn finish(self) -> (u64, String) {
        let digest = self.hasher.finalize();
        (self.bytes, lower_hex(&digest))
    }
}

impl<W: Write> Write for HashingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buf)?;
        self.hasher.update(&buf[..written]);
        self.bytes += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// A file an export wrote.
pub struct Written {
    /// Absolute path.
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: String,
}

/// Check the path a caller names for an export and return the absolute path
/// to write: `extension` is the format's, `input` the canonical path of the
/// model being exported.
pub fn plan(
    config: &Config,
    raw: &str,
    extension: &str,
    overwrite: bool,
    input: &Path,
) -> Result<PathBuf, String> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err("out_path is empty".to_string());
    }
    let mut path = PathBuf::from(raw);
    if let Some(dir) = &config.output_dir
        && path.is_relative()
    {
        path = dir.join(path);
    }
    let has_extension = path
        .extension()
        .and_then(|found| found.to_str())
        .is_some_and(|found| found.eq_ignore_ascii_case(extension));
    if !has_extension {
        return Err(format!(
            "out_path must end in .{extension}: {}",
            path.display()
        ));
    }
    let Some(file_name) = path.file_name() else {
        return Err(format!("out_path names no file: {}", path.display()));
    };
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let parent = std::fs::canonicalize(parent).map_err(|error| {
        format!(
            "the folder for out_path does not exist or cannot be read: {}: {error}",
            parent.display()
        )
    })?;
    if !parent.is_dir() {
        return Err(format!("{} is not a folder", parent.display()));
    }
    let target = parent.join(file_name);
    if let Some(dir) = &config.output_dir
        && !target.starts_with(dir)
    {
        return Err(format!(
            "out_path must be inside {}, the folder the server was started with (--output-dir): {}",
            dir.display(),
            target.display()
        ));
    }
    if target == input || std::fs::canonicalize(&target).is_ok_and(|existing| existing == input) {
        return Err(format!(
            "out_path is the model being read, {}; name another file",
            input.display()
        ));
    }
    if let Ok(metadata) = std::fs::symlink_metadata(&target) {
        if metadata.is_dir() {
            return Err(format!("out_path is a folder: {}", target.display()));
        }
        if !overwrite {
            return Err(format!(
                "{} exists; pass overwrite=true to replace it, or name another file",
                target.display()
            ));
        }
    }
    Ok(target)
}

/// Write the file `fill` produces at `path` (from [`plan`]): beside the
/// target as `<target>.partial`, then renamed over it. `fill` writes through
/// a [`HashingWriter`], so the result carries the size and SHA-256 of what
/// was written.
pub fn write(
    path: &Path,
    overwrite: bool,
    fill: impl FnOnce(&mut HashingWriter<File>) -> io::Result<()>,
) -> Result<Written, String> {
    let mut partial = path.as_os_str().to_os_string();
    partial.push(".partial");
    let partial = PathBuf::from(partial);
    let file = File::create(&partial)
        .map_err(|error| format!("cannot write {}: {error}", partial.display()))?;
    let mut writer = HashingWriter::new(file);
    let outcome = fill(&mut writer).and_then(|()| writer.flush());
    if let Err(error) = outcome {
        let _ = std::fs::remove_file(&partial);
        return Err(format!("cannot write {}: {error}", path.display()));
    }
    let (bytes, sha256) = writer.finish();
    if !overwrite && std::fs::symlink_metadata(path).is_ok() {
        let _ = std::fs::remove_file(&partial);
        return Err(format!(
            "{} appeared while the export was written; pass overwrite=true to replace it",
            path.display()
        ));
    }
    if let Err(error) = std::fs::rename(&partial, path) {
        let _ = std::fs::remove_file(&partial);
        return Err(format!("cannot write {}: {error}", path.display()));
    }
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    Ok(Written {
        path,
        bytes,
        sha256,
    })
}

/// The `file://` URI of an absolute path, percent-encoded.
pub fn file_uri(path: &Path) -> String {
    let mut text = path.to_string_lossy().into_owned();
    if cfg!(windows) {
        text = text.replace('\\', "/");
        let stripped = text.strip_prefix("//?/").map(str::to_string);
        if let Some(stripped) = stripped {
            text = stripped;
        }
    }
    let mut uri = String::from("file://");
    if !text.starts_with('/') {
        uri.push('/');
    }
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' | b'/' | b':' => {
                uri.push(char::from(byte));
            }
            other => uri.push_str(&format!("%{other:02X}")),
        }
    }
    uri
}
