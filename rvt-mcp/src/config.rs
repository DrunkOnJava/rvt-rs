//! What the server is started with, and how a tool job fails.

use std::path::PathBuf;

use rvt::reader::OpenLimits;
use rvt::walker::WalkerLimits;

/// Settings fixed when the server starts. The resource limits are the
/// ones the CLIs apply, so a model that `rvt-ifc` refuses is refused here too.
#[derive(Debug, Clone)]
pub struct Config {
    pub open_limits: OpenLimits,
    pub walker_limits: WalkerLimits,
    /// How many decoded models the server keeps; the least recently used one
    /// is dropped when another is opened.
    pub max_open_models: usize,
    /// When set, exports are written inside this folder only.
    pub output_dir: Option<PathBuf>,
}

/// How a tool job ends when it does not produce an answer.
#[derive(Debug)]
pub enum Failure {
    /// The client cancelled the request; nothing partial was written.
    Cancelled,
    /// The message the tool result carries, with `isError` set.
    Message(String),
}

impl From<String> for Failure {
    fn from(message: String) -> Self {
        Self::Message(message)
    }
}
