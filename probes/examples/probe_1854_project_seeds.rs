//! #1854: which identity of a project holds across its saves and differs
//! between projects.
//!
//! FACT under test: rvt-rs derives every GlobalId Revit gives no entity from
//! the document's `Unique Document GUID` (`BasicFileInfo`), and that GUID
//! differs on every yearly save of every Autodesk sample project, so those
//! GlobalIds move between saves. A seed for them must hold across the saves of
//! one project and differ between projects, including two projects started
//! from one template. The probe prints each candidate, hashed, so that equal
//! and different values can be compared across files without printing the
//! value itself:
//! - `document`: the `Unique Document GUID`, the seed today;
//! - `first_episode`: the GUID of the document history's first editing
//!   episode (`Global/History`), with the number of episodes;
//! - `project_info`: the Revit GlobalId of the `ProjectInfo` element, rebuilt
//!   from its UniqueId (RE-48), as Revit's exporter seeds project-level GUIDs
//!   from it (revit-ifc `GUIDUtil.CreateProjectLevelGUID`);
//! - `project_identity_guid` and `project_identity_model`: the `m_guid` and
//!   `m_modelIdentity` of the `ProjectIdentity` record.
//!
//! Output, one line each, values as the first 12 hex digits of their SHA-256
//! (`-` where the file has none):
//!   seeds document=<h> first_episode=<h> episodes=<n> project_info=<h> project_identity_guid=<h> project_identity_model=<h>
//!
//! Usage:
//!   cargo run --profile ci -p rvt-probes --example probe_1854_project_seeds -- MODEL.rvt

use rvt::research::native_document::{NATIVE_RELEASES, Options, extract_graphs};
use rvt::research::revit_global_ids::{EpisodeHistory, revit_global_ids};
use rvt::{RevitFile, compression, streams};
use sha2::{Digest, Sha256};

fn open(path: &str) -> anyhow::Result<RevitFile> {
    Ok(RevitFile::open_with_limits(
        path,
        rvt::cli::open_limits(None, None, false),
    )?)
}

fn hashed(value: Option<&str>) -> String {
    match value {
        Some(value) => Sha256::digest(value.as_bytes())[..6]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
        None => "-".to_string(),
    }
}

/// `Global/History` inflated, as `revit_global_ids` reads it.
fn history(rf: &mut RevitFile) -> Option<Vec<u8>> {
    let stored = rf.read_stream(streams::GLOBAL_HISTORY).ok()?;
    let prepared = compression::prepare_stream_for_inflate(streams::GLOBAL_HISTORY, &stored);
    let data = prepared.as_ref();
    [0, 4, 8, 16]
        .into_iter()
        .filter(|&offset| compression::has_gzip_magic(data, offset))
        .find_map(|offset| compression::inflate_at(data, offset).ok())
        .or_else(|| compression::inflate_at_auto(data).ok().map(|(_, out)| out))
}

fn main() -> anyhow::Result<()> {
    // Measure passes a single sample `--records` too.
    let path = std::env::args()
        .skip(1)
        .find(|arg| !arg.starts_with("--"))
        .expect("usage: probe_1854_project_seeds MODEL.rvt");
    let mut rf = open(&path)?;

    let document = rf
        .basic_file_info()
        .ok()
        .and_then(|info| info.document_guid().map(str::to_owned));

    let episodes = history(&mut rf).and_then(|bytes| EpisodeHistory::parse(&bytes));
    let first_episode = episodes
        .as_ref()
        .and_then(|history| history.episode_guid(0))
        .map(|guid| {
            guid.iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        });

    let mut project_info_ids = Vec::new();
    let mut identity_guid = None;
    let mut identity_model = None;
    let options = Options {
        selected_classes: ["ProjectInfo", "ProjectIdentity"]
            .iter()
            .map(|class| (*class).to_string())
            .collect(),
        releases: NATIVE_RELEASES,
        ..Options::default()
    };
    extract_graphs(&mut open(&path)?, &options, |record| {
        match record.class_name.as_deref() {
            Some("ProjectInfo") => project_info_ids.push(record.identity.element_id),
            Some("ProjectIdentity") => {
                let root = record
                    .graph
                    .as_ref()
                    .and_then(|graph| graph.objects.first());
                let field = |name: &str| {
                    root.and_then(|root| root.fields.get(name))
                        .and_then(|value| serde_json::to_string(value).ok())
                };
                identity_guid = field("m_guid");
                identity_model = field("m_modelIdentity");
            }
            _ => {}
        }
        Ok(())
    })?;
    let ids = revit_global_ids(&mut open(&path)?).unwrap_or_default();
    let project_info = match project_info_ids.as_slice() {
        [one] => u32::try_from(*one)
            .ok()
            .and_then(|id| ids.get(&id).cloned()),
        _ => None,
    };

    println!(
        "seeds document={} first_episode={} episodes={} project_info={} project_info_records={} project_identity_guid={} project_identity_model={}",
        hashed(document.as_deref()),
        hashed(first_episode.as_deref()),
        episodes.as_ref().map_or(0, EpisodeHistory::len),
        hashed(project_info.as_deref()),
        project_info_ids.len(),
        hashed(identity_guid.as_deref()),
        hashed(identity_model.as_deref())
    );
    Ok(())
}
