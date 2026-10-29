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
/// in total ([`crate::compression::InflateLimits::max_stream_total_bytes`]):
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
