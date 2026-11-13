//! The one folder the server reads models from and writes exports to (#1883).
//!
//! What a Revit file holds (names, values, messages) reaches the client's
//! model unmarked, so a crafted file can steer that model to ask the server for
//! other files: to open one that is not the project's, or to overwrite one.
//! The server therefore acts inside one folder, the one its setup picks, until
//! `--unconfined` lifts the limit.
//!
//! The setup picks it with `--root <folder>`, or the environment variable
//! `RVT_MCP_ROOT` for a client that passes only an environment. With neither,
//! the folder the server was started in is the root, unless that is too broad
//! to be one: an MCP client chooses a server's working directory, and some
//! start theirs from `/`. A working directory that is the top of a file
//! system, the home folder, or a folder that holds the home folder gives no
//! root: the server starts, and every tool that touches a file refuses with a
//! message that names `--root`. Reaching a whole file system stays something
//! the setup types (`--root /`, or `--unconfined`).
//!
//! A path is compared with the folder after both are made canonical, so `..`
//! and symbolic links cannot leave it. A path whose text alone puts it outside
//! is refused before any file is looked at, so that a refusal does not say
//! whether the file exists.
//!
//! What this does not do: defend against another process that rewrites the
//! links inside the folder while a call runs.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use crate::bounds;

/// The environment variable that names the root for a client that cannot pass
/// an argument.
pub const ROOT_ENV: &str = "RVT_MCP_ROOT";

/// A folder files are confined to: its path with every link resolved, which
/// is what a resolved path is held to, and the other spellings of it that a
/// client may use. A folder reached through a link has two names (`/tmp/work`
/// and `/private/tmp/work` on macOS, a symlinked `/home` on Linux), and a
/// client sends the one it knows.
#[derive(Debug, Clone)]
pub struct Folder {
    canonical: PathBuf,
    spellings: Vec<PathBuf>,
}

/// The root: the folder models are read from and exports written to.
#[derive(Debug, Clone)]
pub struct Root {
    folder: Folder,
}

/// Where the server may read and write.
#[derive(Debug, Clone)]
pub enum Reach {
    /// Inside one folder.
    Folder(Root),
    /// Nowhere: no root was picked and the working directory is too broad to be
    /// one (`why` says how). Every tool that touches a file refuses.
    Unset {
        started_in: PathBuf,
        why: &'static str,
    },
    /// `--unconfined`: any file the account can read, and exports anywhere.
    Anywhere,
}

impl Folder {
    /// The folder `given`, which must exist. Its spellings are `given` itself,
    /// made absolute and cleaned of `.` and `..`, and the shell's `PWD` where
    /// that names this folder.
    pub fn new(given: &Path) -> Result<Self, String> {
        let canonical = std::fs::canonicalize(given).map_err(|error| error.to_string())?;
        if !canonical.is_dir() {
            return Err(format!("{} is not a folder", canonical.display()));
        }
        let mut spellings: Vec<PathBuf> = Vec::new();
        let mut spell = |path: PathBuf| {
            let flat = lexical(&path);
            if flat.is_absolute() && flat != canonical && !spellings.contains(&flat) {
                spellings.push(flat);
            }
        };
        spell(if given.is_absolute() {
            given.to_path_buf()
        } else {
            std::env::current_dir().unwrap_or_default().join(given)
        });
        if let Some(pwd) = std::env::var_os("PWD") {
            let pwd = PathBuf::from(pwd);
            if std::fs::canonicalize(&pwd).is_ok_and(|resolved| resolved == canonical) {
                spell(pwd);
            }
        }
        Ok(Self {
            canonical,
            spellings,
        })
    }

    /// The folder with every link resolved.
    pub fn dir(&self) -> &Path {
        &self.canonical
    }

    /// Whether the absolute `path` lies inside the folder by its text alone,
    /// under its resolved path or under another spelling of it. No file is
    /// looked at, so a link is not followed: the caller still resolves the path
    /// and checks it with [`Folder::holds`], which is the gate.
    pub fn lexically_holds(&self, path: &Path) -> bool {
        let flat = lexical(path);
        flat.starts_with(&self.canonical)
            || self
                .spellings
                .iter()
                .any(|spelling| flat.starts_with(spelling))
    }

    /// Whether `resolved`, a path with every link resolved, is inside the folder.
    pub fn holds(&self, resolved: &Path) -> bool {
        resolved.starts_with(&self.canonical)
    }
}

impl Root {
    /// The folder `given`, which must exist.
    pub fn new(given: &Path) -> Result<Self, String> {
        Ok(Self {
            folder: Folder::new(given)?,
        })
    }

    pub fn folder(&self) -> &Folder {
        &self.folder
    }

    pub fn dir(&self) -> &Path {
        self.folder.dir()
    }

    /// `raw` as an absolute path to look at: taken from the root when it is
    /// relative, and refused when its text alone leaves the root. `what` names
    /// the argument in the refusal. The path still has to be made canonical and
    /// checked with [`Root::holds`]: a link inside the root can lead out.
    pub fn locate(&self, raw: &Path, what: &str) -> Result<PathBuf, String> {
        let joined = if raw.is_relative() {
            self.dir().join(raw)
        } else {
            raw.to_path_buf()
        };
        if self.folder.lexically_holds(&joined) {
            Ok(joined)
        } else {
            Err(self.refusal(what, raw))
        }
    }

    /// Whether `canonical`, a path with every link resolved, is inside the root.
    pub fn holds(&self, canonical: &Path) -> bool {
        self.folder.holds(canonical)
    }

    /// The message for a path outside the root: where the root is and how to
    /// move it.
    pub fn refusal(&self, what: &str, raw: &Path) -> String {
        bounds::message(format!(
            "{what} {} is outside {}, the folder this server reads from and writes to. Name a path inside it. The person who set the server up can start it with --root <folder> to use another folder, or with --unconfined to lift the limit.",
            raw.display(),
            self.dir().display()
        ))
    }
}

impl Reach {
    /// What the setup gives: `--unconfined`, else `--root`, else
    /// `RVT_MCP_ROOT` (an empty value is no value), else the working directory
    /// where that is not too broad ([`too_broad`]).
    pub fn choose(
        unconfined: bool,
        flag: Option<&Path>,
        env: Option<&OsStr>,
        cwd: std::io::Result<PathBuf>,
        homes: &[PathBuf],
    ) -> Result<Self, String> {
        if unconfined {
            return Ok(Self::Anywhere);
        }
        let named = flag.map(|dir| (dir, "--root")).or_else(|| {
            env.filter(|value| !value.is_empty())
                .map(|value| (Path::new(value), ROOT_ENV))
        });
        if let Some((dir, from)) = named {
            return Root::new(dir)
                .map(Self::Folder)
                .map_err(|error| format!("{from} {}: {error}", dir.display()));
        }
        let cwd = cwd.map_err(|error| {
            format!(
                "cannot find the folder the server was started in: {error}. Pass --root <folder>, or set {ROOT_ENV}."
            )
        })?;
        let started_in = std::fs::canonicalize(&cwd).unwrap_or_else(|_| cwd.clone());
        match too_broad(&started_in, homes) {
            Some(why) => Ok(Self::Unset { started_in, why }),
            None => Root::new(&cwd).map(Self::Folder).map_err(|error| {
                format!(
                    "the folder the server was started in, {}: {error}",
                    cwd.display()
                )
            }),
        }
    }

    /// The folder files are confined to: none when unconfined, and an error
    /// (the setup's to fix, naming `--root`) when no root was picked and the
    /// working directory is too broad to be one.
    pub fn limit(&self) -> Result<Option<&Root>, String> {
        match self {
            Self::Folder(root) => Ok(Some(root)),
            Self::Anywhere => Ok(None),
            Self::Unset { started_in, why } => Err(format!(
                "This server has no folder to read models from and write exports to: no --root was given (nor {ROOT_ENV}), and it was started in {}, which is {why}. Ask the person who set it up to start it with --root <folder> naming the folder that holds the models (or --unconfined to lift the limit).",
                started_in.display()
            )),
        }
    }
}

/// Why the working directory cannot be the root, when it cannot: the top of a
/// file system (`/`, a drive such as `C:\`), the home folder, or a folder that
/// holds the home folder. `homes` are the home folders, canonical.
pub fn too_broad(cwd: &Path, homes: &[PathBuf]) -> Option<&'static str> {
    if cwd.parent().is_none() {
        return Some("the top of a file system");
    }
    for home in homes {
        if cwd == home {
            return Some("the home folder");
        }
        if home.starts_with(cwd) {
            return Some("a folder that holds the home folder");
        }
    }
    None
}

/// The home folders of the account, canonical: `HOME` and, on Windows,
/// `USERPROFILE`.
pub fn home_folders() -> Vec<PathBuf> {
    ["HOME", "USERPROFILE"]
        .iter()
        .filter_map(std::env::var_os)
        .filter_map(|home| std::fs::canonicalize(home).ok())
        .collect()
}

/// `path` with its `.` and `..` components resolved by its text alone: no file
/// is looked at, so a link is not followed. A `..` at the top of an absolute
/// path stays at the top, and one in a relative path is kept.
fn lexical(path: &Path) -> PathBuf {
    let mut flat = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let last = flat.components().next_back();
                match last {
                    Some(Component::RootDir | Component::Prefix(_)) => {}
                    None | Some(Component::ParentDir) => flat.push(".."),
                    Some(_) => {
                        flat.pop();
                    }
                }
            }
            other => flat.push(other.as_os_str()),
        }
    }
    flat
}

// The ways a path can get out of a root, failure first: a `..` that climbs out;
// a name that only starts like the root's; a `..` above the top of the file
// system; a relative path, which is taken from the root and so can climb out of
// it too; a refusal that does not tell the person where the root is. And the
// ways a working directory can be too broad to be a root: the top of a file
// system, the home folder, a folder above it, and a folder that only starts
// like the home folder's. No file is touched: the root is a name.
#[cfg(all(test, unix))]
mod tests {
    use super::*;

    fn root() -> Root {
        Root {
            folder: Folder {
                canonical: PathBuf::from("/work/project"),
                spellings: Vec::new(),
            },
        }
    }

    /// A root reached through a link: `/tmp/work` is `/private/tmp/work`.
    fn aliased() -> Root {
        Root {
            folder: Folder {
                canonical: PathBuf::from("/private/tmp/work"),
                spellings: vec![PathBuf::from("/tmp/work")],
            },
        }
    }

    fn located(raw: &str) -> Result<PathBuf, String> {
        root().locate(Path::new(raw), "model")
    }

    #[test]
    fn a_dot_dot_that_climbs_out_of_the_root_is_refused() {
        assert!(located("/work/project/a/../../elsewhere/x.rvt").is_err());
        assert!(located("/work/project/../project2/x.rvt").is_err());
        assert!(located("../x.rvt").is_err());
        assert!(located("a/../../x.rvt").is_err());
    }

    #[test]
    fn a_folder_that_only_starts_like_the_root_is_outside() {
        assert!(located("/work/project2/x.rvt").is_err());
        assert!(located("/work/projectx").is_err());
        assert!(located("/work").is_err());
    }

    #[test]
    fn a_dot_dot_above_the_top_of_the_file_system_stays_at_the_top() {
        assert_eq!(lexical(Path::new("/../x")), PathBuf::from("/x"));
        assert_eq!(lexical(Path::new("/a/../../../x")), PathBuf::from("/x"));
        // As the file system reads them: `/..` is `/`.
        assert!(located("/../work/project/x.rvt").is_ok());
        assert!(located("/../../etc/hosts").is_err());
    }

    #[test]
    fn a_relative_dot_dot_that_never_comes_back_is_kept() {
        assert_eq!(lexical(Path::new("../../x")), PathBuf::from("../../x"));
        assert_eq!(lexical(Path::new("a/../../x")), PathBuf::from("../x"));
    }

    #[test]
    fn paths_inside_are_taken_from_the_root_and_cleaned_of_dots() {
        assert_eq!(
            located("x.rvt").unwrap(),
            PathBuf::from("/work/project/x.rvt")
        );
        assert!(located("./a//b/../c.rvt").is_ok());
        assert!(located("/work/project/./a/c.rvt").is_ok());
        assert!(located("/work/project").is_ok());
        assert!(located("/work/project/").is_ok());
    }

    #[test]
    fn a_refusal_names_the_root_and_both_ways_to_move_it() {
        let message = located("/etc/hosts").unwrap_err();
        assert!(message.contains("/work/project"), "{message}");
        assert!(message.contains("--root"), "{message}");
        assert!(message.contains("--unconfined"), "{message}");
    }

    #[test]
    fn a_folder_reached_through_a_link_takes_paths_spelled_either_way() {
        let root = aliased();
        for inside in [
            "/tmp/work/x.rvt",
            "/private/tmp/work/x.rvt",
            "/tmp/work/a/../b/x.rvt",
        ] {
            assert!(root.locate(Path::new(inside), "model").is_ok(), "{inside}");
        }
        // Still outside by either name: another folder, a name that only starts
        // like one, a `..` that climbs out of either.
        for outside in [
            "/tmp/other/x.rvt",
            "/tmp/workshop/x.rvt",
            "/private/tmp/other/x.rvt",
            "/tmp/work/../other/x.rvt",
            "/private/tmp/work/../x.rvt",
            "/tmp/x.rvt",
        ] {
            let message = root.locate(Path::new(outside), "model").unwrap_err();
            assert!(
                message.contains("/private/tmp/work"),
                "{outside}: {message}"
            );
        }
        // The resolved path is the gate and is held to the resolved name alone.
        assert!(root.holds(Path::new("/private/tmp/work/x.rvt")));
        assert!(!root.holds(Path::new("/tmp/work/x.rvt")));
    }

    #[test]
    fn a_resolved_path_is_held_only_by_its_components() {
        let root = root();
        assert!(root.holds(Path::new("/work/project/a/b.rvt")));
        assert!(!root.holds(Path::new("/work/project2/b.rvt")));
        assert!(!root.holds(Path::new("/work")));
    }

    fn homes() -> Vec<PathBuf> {
        vec![PathBuf::from("/people/alex")]
    }

    #[test]
    fn a_working_directory_that_holds_everything_or_the_home_folder_is_too_broad() {
        assert_eq!(
            too_broad(Path::new("/"), &homes()),
            Some("the top of a file system")
        );
        assert_eq!(
            too_broad(Path::new("/"), &[]),
            Some("the top of a file system")
        );
        assert_eq!(
            too_broad(Path::new("/people/alex"), &homes()),
            Some("the home folder")
        );
        assert_eq!(
            too_broad(Path::new("/people"), &homes()),
            Some("a folder that holds the home folder")
        );
    }

    #[test]
    fn a_folder_inside_the_home_folder_or_beside_it_is_not_too_broad() {
        assert_eq!(
            too_broad(Path::new("/people/alex/Projects/house"), &homes()),
            None
        );
        // Starts like the home folder's name, is not it, and does not hold it.
        assert_eq!(too_broad(Path::new("/people/alex2"), &homes()), None);
        assert_eq!(too_broad(Path::new("/people/alexis"), &homes()), None);
        assert_eq!(too_broad(Path::new("/srv/models"), &homes()), None);
    }

    #[test]
    fn no_root_picked_and_a_broad_working_directory_leaves_the_reach_unset() {
        let unset = Reach::choose(false, None, None, Ok(PathBuf::from("/")), &homes()).unwrap();
        let message = unset.limit().unwrap_err();
        assert!(message.contains("--root"), "{message}");
        assert!(message.contains(ROOT_ENV), "{message}");
        assert!(message.contains("the top of a file system"), "{message}");
    }

    #[test]
    fn unconfined_wins_over_everything_and_a_picked_root_over_the_working_directory() {
        let anywhere = Reach::choose(true, Some(Path::new("/nope")), None, Ok("/".into()), &[]);
        assert!(matches!(anywhere, Ok(Reach::Anywhere)));
        // A root that is not there is an error, not a fall back to the working directory.
        let missing = Reach::choose(
            false,
            Some(Path::new("/definitely/not/here")),
            None,
            Ok(PathBuf::from("/work/project")),
            &homes(),
        );
        assert!(missing.is_err());
        // An empty environment value is no value.
        let empty = Reach::choose(
            false,
            None,
            Some(OsStr::new("")),
            Ok(PathBuf::from("/")),
            &homes(),
        )
        .unwrap();
        assert!(empty.limit().is_err());
    }
}
