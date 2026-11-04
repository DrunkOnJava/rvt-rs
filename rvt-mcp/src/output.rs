//! Where an export goes and how it is written.
//!
//! An export is the one thing the server writes. It goes to the path the
//! caller names, which must carry the format's extension, must not be the
//! model that was read, and must not exist unless the caller passes
//! `overwrite`. It is written to a temporary file beside its target and put in
//! place when whole, so a failed or cancelled export never leaves a truncated
//! file under the target's name.

use std::io::{self, Write};
use std::path::{Path, PathBuf};

use rvt::control::WalkerControl;
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;

use crate::config::{Config, Failure};
use crate::store::lower_hex;

/// A writer over the temporary file that counts and hashes what passes
/// through it, and stops writing once the request is cancelled.
pub struct HashingWriter<'a> {
    inner: NamedTempFile,
    hasher: Sha256,
    bytes: u64,
    control: &'a WalkerControl,
}

impl<'a> HashingWriter<'a> {
    fn new(inner: NamedTempFile, control: &'a WalkerControl) -> Self {
        Self {
            inner,
            hasher: Sha256::new(),
            bytes: 0,
            control,
        }
    }

    fn finish(self) -> (NamedTempFile, u64, String) {
        let digest = self.hasher.finalize();
        (self.inner, self.bytes, lower_hex(&digest))
    }
}

impl Write for HashingWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Not `Interrupted`: `write_all` retries on that, and would never stop.
        if self.control.check().is_err() {
            return Err(io::Error::other("the request was cancelled"));
        }
        let written = self.inner.as_file_mut().write(buf)?;
        self.hasher.update(&buf[..written]);
        self.bytes += written as u64;
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.as_file_mut().flush()
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

/// Write the file `fill` produces at `target` (from [`plan`]).
///
/// The bytes go to a uniquely named temporary file in the target's folder,
/// which `tempfile` creates exclusively (so it cannot follow a planted link,
/// and two exports to one target do not share it) and removes on every error
/// path. When whole it is put in place: with `overwrite` it replaces the
/// target; without, it is linked there and fails if the target exists, so a
/// file that appeared meanwhile is never replaced. `fill` writes through a
/// [`HashingWriter`], so the result carries the size and SHA-256 of what was
/// written. A cancelled `control` stops the write at its next chunk and
/// leaves nothing under the target's name.
pub fn write(
    target: &Path,
    overwrite: bool,
    control: &WalkerControl,
    fill: impl FnOnce(&mut HashingWriter<'_>) -> io::Result<()>,
) -> Result<Written, Failure> {
    let parent = match target.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };
    let file = tempfile::Builder::new()
        .prefix(".rvt-rs-mcp-")
        .suffix(".partial")
        .tempfile_in(parent)
        .map_err(|error| {
            Failure::Message(format!("cannot write in {}: {error}", parent.display()))
        })?;
    // The temporary file is private (mode 0600); an export is an ordinary file.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = file
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o644));
    }
    let mut writer = HashingWriter::new(file, control);
    let outcome = fill(&mut writer).and_then(|()| writer.flush());
    if let Err(error) = outcome {
        drop(writer);
        return Err(if control.check().is_err() {
            Failure::Cancelled
        } else {
            Failure::Message(format!("cannot write {}: {error}", target.display()))
        });
    }
    let (file, bytes, sha256) = writer.finish();
    if control.check().is_err() {
        return Err(Failure::Cancelled);
    }
    let placed = if overwrite {
        file.persist(target)
    } else {
        file.persist_noclobber(target)
    };
    if let Err(error) = placed {
        let message = if error.error.kind() == io::ErrorKind::AlreadyExists {
            format!(
                "{} exists; pass overwrite=true to replace it, or name another file",
                target.display()
            )
        } else {
            format!("cannot write {}: {}", target.display(), error.error)
        };
        return Err(Failure::Message(message));
    }
    let path = std::fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf());
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
