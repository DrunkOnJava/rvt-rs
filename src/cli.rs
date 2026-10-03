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
