//! Behaviour shared by the shipped command-line tools.

/// Make a CLI exit quietly when its standard output is closed early —
/// `rvt-schema model.rvt | head` — instead of panicking with
/// `failed printing to stdout: Broken pipe` and a backtrace hint.
///
/// Rust ignores `SIGPIPE`, so `println!` into a closed pipe panics. This
/// installs a panic hook that turns exactly that panic into exit status 141
/// (what a shell reports for a process killed by `SIGPIPE`) and leaves every
/// other panic to the default report. Call it first thing in `main`.
pub fn exit_quietly_on_broken_pipe() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let message = payload
            .downcast_ref::<String>()
            .map(String::as_str)
            .or_else(|| payload.downcast_ref::<&str>().copied())
            .unwrap_or_default();
        if is_closed_stdout_pipe(message) {
            std::process::exit(141);
        }
        default_hook(info);
    }));
}

/// `println!`'s panic message for a write into a closed pipe: `EPIPE`
/// (`os error 32`) on Unix; on Windows `ERROR_BROKEN_PIPE` (`os error 109`,
/// "The pipe has been ended" — what the CI runner reports) or
/// `ERROR_NO_DATA` (`os error 232`, "The pipe is being closed"). Other stdout
/// failures (a full disk behind a redirect) are not swallowed.
fn is_closed_stdout_pipe(message: &str) -> bool {
    message.starts_with("failed printing to stdout")
        && ["os error 32)", "os error 109)", "os error 232)"]
            .iter()
            .any(|code| message.contains(code))
}

/// The `--large-model` preset's per-stream cap: the whole file's cap
/// ([`crate::reader::DEFAULT_MAX_FILE_BYTES`]). A stream is never larger
/// than its file, which is already read into memory, so this adds no
/// allocation an attacker could not force with the file itself.
pub const LARGE_MODEL_MAX_STREAM_BYTES: u64 = crate::reader::DEFAULT_MAX_FILE_BYTES;

/// The `--large-model` preset's inflate cap, per gzip member and per stream
/// in total ([`crate::compression::InflateLimits::max_stream_total_bytes_for`]):
/// 8 GiB, or the address space on a 32-bit target. Client models with a
/// 1 GiB stored partition inflate past 1 GiB (#1075).
pub const LARGE_MODEL_MAX_INFLATE_BYTES: usize = if usize::BITS >= 64 {
    (8u64 << 30) as usize
} else {
    usize::MAX
};

/// The [`crate::reader::OpenLimits`] a CLI opens its input with: the
/// defaults, the `--large-model` preset over them, and an explicit
/// `--max-stream-bytes` / `--max-inflate-bytes` over both.
pub fn open_limits(
    max_stream_bytes: Option<u64>,
    max_inflate_bytes: Option<usize>,
    large_model: bool,
) -> crate::reader::OpenLimits {
    let mut limits = crate::reader::OpenLimits::default();
    if large_model {
        limits.max_stream_bytes = LARGE_MODEL_MAX_STREAM_BYTES;
        limits.inflate_limits.max_output_bytes = LARGE_MODEL_MAX_INFLATE_BYTES;
    }
    if let Some(bytes) = max_stream_bytes {
        limits.max_stream_bytes = bytes;
    }
    if let Some(bytes) = max_inflate_bytes {
        limits.inflate_limits.max_output_bytes = bytes;
    }
    limits
}

/// The lines a CLI prints when a resource limit kept streams from being read
/// whole (#1075), one per stream, then how to go on.
pub fn unread_stream_report(unread: &[crate::reader::UnreadStream]) -> Vec<String> {
    let mut lines: Vec<String> = unread.iter().map(|stream| format!("  {stream}")).collect();
    lines.push(
        "  Re-run with --large-model (or a larger --max-stream-bytes / --max-inflate-bytes) to read them, or --allow-partial to write the incomplete output anyway."
            .to_string(),
    );
    lines
}

/// The warning lines a CLI prints when readers stopped at a work budget
/// (#1833), one per reader with how often, then what that means
/// ([`crate::ifc::ExportDiagnostics::work_budget_stops`]). The output is
/// written all the same; no option raises a work budget.
pub fn work_budget_report(stops: &std::collections::BTreeMap<String, usize>) -> Vec<String> {
    let mut lines: Vec<String> = stops
        .iter()
        .map(|(reader, times)| format!("  {reader} stopped {times} time(s)"))
        .collect();
    lines.push(
        "  The output is written without it. No option raises a work budget: only a file built to make a reader do unbounded work is known to reach one."
            .to_string(),
    );
    lines
}

/// Whether `a` and `b` are one file: the same path once links, `.` and `..`
/// are resolved, or two names of one file (a hard link on Unix). A path that
/// does not exist is no existing file's, so an output not yet written is never
/// the input. A CLI that writes beside its input checks this before it writes,
/// since comparing the paths as typed lets `./model.rvt` and a symlink through.
pub fn same_file(a: &std::path::Path, b: &std::path::Path) -> bool {
    if matches!(
        (std::fs::canonicalize(a), std::fs::canonicalize(b)),
        (Ok(x), Ok(y)) if x == y
    ) {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(a), Ok(b)) = (std::fs::metadata(a), std::fs::metadata(b)) {
            return a.dev() == b.dev() && a.ino() == b.ino();
        }
    }
    false
}

/// A text cell of a CSV a spreadsheet may open: quoted when it holds a comma, a
/// quote or a line break (RFC 4180), and led by `'` when it starts with `=`,
/// `+`, `-`, `@`, a tab or a carriage return, which a spreadsheet would read as
/// a formula. The text of a crafted file (a title, a user name, a path) reaches
/// such a cell, so a spreadsheet must read it as the text it is. Numbers are
/// written with `to_string`, not through this.
pub fn csv_text(value: &str) -> String {
    let value = if value.starts_with(['=', '+', '-', '@', '\t', '\r']) {
        std::borrow::Cow::Owned(format!("'{value}"))
    } else {
        std::borrow::Cow::Borrowed(value)
    };
    if value.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::csv_text;

    #[test]
    fn a_formula_is_led_by_a_quote() {
        for lead in ["=", "+", "-", "@", "\t"] {
            let cell = format!("{lead}1+1");
            assert_eq!(csv_text(&cell), format!("'{cell}"), "led by {lead:?}");
        }
    }

    #[test]
    fn a_leading_carriage_return_is_led_and_then_quoted() {
        assert_eq!(csv_text("\r=1"), "\"'\r=1\"");
    }

    #[test]
    fn a_neutralised_cell_is_still_quoted_for_its_commas_and_quotes() {
        assert_eq!(
            csv_text("=HYPERLINK(\"x\",\"y\")"),
            "\"'=HYPERLINK(\"\"x\"\",\"\"y\"\")\""
        );
    }

    #[test]
    fn text_that_is_not_a_formula_is_left_as_it_is() {
        for plain in ["", "title", "a=b", "x-1", "'=1", "12"] {
            assert_eq!(csv_text(plain), plain);
        }
    }

    #[test]
    fn commas_quotes_and_line_breaks_are_quoted() {
        assert_eq!(csv_text("a,b"), "\"a,b\"");
        assert_eq!(csv_text("say \"hi\""), "\"say \"\"hi\"\"\"");
        assert_eq!(csv_text("a\nb"), "\"a\nb\"");
    }
}
