//! How much one answer may carry (#1883).
//!
//! An answer is sized by the file and the client, and a crafted file can make
//! any piece of text in it, or any list, as long as it likes. Three bounds hold
//! whatever the file holds: text from the file is cut at `--max-field-chars`
//! characters, the diagnostics document (the one answer no `limit` pages) is
//! held to a fixed number of entries a list, and an answer over
//! `--max-answer-bytes` is refused and not sent. Whatever is cut says so in the
//! text it leaves, or in the answer's `notes`.

use std::io::{self, Write};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::Serialize;
use serde_json::Value;

/// The default for `--max-field-chars`: far above any name Revit lets a person
/// type, so a model that is not crafted never meets it.
pub const DEFAULT_FIELD_CHARS: usize = 512;

/// The default for `--max-answer-bytes`: a page of 1000 elements is about one
/// megabyte.
pub const DEFAULT_ANSWER_BYTES: usize = 4 * 1024 * 1024;

/// The most entries of one list or object a document from the file keeps.
const ENTRIES: usize = 5_000;

/// What `--max-field-chars` was started with. A single process-wide value, set
/// once before the first request, so that the places that put text from the file
/// into an answer need no handle to the configuration.
static FIELD_CHARS: AtomicUsize = AtomicUsize::new(DEFAULT_FIELD_CHARS);

pub fn set_field_chars(chars: usize) {
    FIELD_CHARS.store(chars.max(1), Ordering::Relaxed);
}

fn field_chars() -> usize {
    FIELD_CHARS.load(Ordering::Relaxed)
}

/// `text` whole up to `limit` characters, otherwise its first `limit`
/// characters and how many were left out, so that a client sees the cut.
fn clip_to(text: &str, limit: usize) -> String {
    match text.char_indices().nth(limit) {
        None => text.to_string(),
        Some((at, _)) => format!(
            "{}…[+{} characters]",
            &text[..at],
            text[at..].chars().count()
        ),
    }
}

/// A piece of text from the file as an answer carries it.
pub fn clip(text: &str) -> String {
    clip_to(text, field_chars())
}

/// [`clip`] for text already owned, which stays as it is when it is short.
pub fn clip_owned(text: String) -> String {
    let limit = field_chars();
    if text.len() <= limit {
        text
    } else {
        clip_to(&text, limit)
    }
}

/// A message that can quote the file (a phase list, a stream name, a decoder's
/// error). It may be longer than a field, since it can list a file's phases.
pub fn message(text: String) -> String {
    let limit = field_chars().saturating_mul(8);
    if text.len() <= limit {
        text
    } else {
        clip_to(&text, limit)
    }
}

/// A document built from the file, bounded in place: every string is clipped
/// and every list or object keeps at most [`ENTRIES`] entries. What was cut is
/// named in `notes`.
pub fn bound_value(value: &mut Value, path: &str, notes: &mut Vec<String>) {
    match value {
        Value::String(text) => {
            if text.len() > field_chars() {
                *text = clip_to(text, field_chars());
            }
        }
        Value::Array(items) => {
            if items.len() > ENTRIES {
                notes.push(format!(
                    "{path}: the first {ENTRIES} of {} entries are kept",
                    items.len()
                ));
                items.truncate(ENTRIES);
            }
            let child = format!("{path}[]");
            for item in items.iter_mut() {
                bound_value(item, &child, notes);
            }
        }
        Value::Object(entries) => {
            if entries.len() > ENTRIES {
                notes.push(format!(
                    "{path}: the first {ENTRIES} of {} entries are kept",
                    entries.len()
                ));
                let extra: Vec<String> = entries.keys().skip(ENTRIES).cloned().collect();
                for key in extra {
                    entries.remove(&key);
                }
            }
            for (key, item) in entries.iter_mut() {
                bound_value(item, &format!("{path}.{key}"), notes);
            }
        }
        _ => {}
    }
}

/// A writer that keeps nothing and counts what it was given.
struct Counted(usize);

impl Write for Counted {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The size of `answer` as JSON, counted without keeping it; `None` if it does
/// not serialize.
pub fn json_bytes(answer: &impl Serialize) -> Option<usize> {
    let mut count = Counted(0);
    serde_json::to_writer(&mut count, answer).ok()?;
    Some(count.0)
}
