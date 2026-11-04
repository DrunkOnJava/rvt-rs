//! The two tools that write a file: `export_ifc` and `export_gltf`.
//!
//! Each writes what `rvt-ifc` and `rvt-gltf` write from the same model, to the
//! path the caller names, and returns a summary of what went in and what was
//! left out, with the SHA-256 of the file. The file itself is never returned.

use std::io::Write;

use rmcp::model::Resource;
use rvt::control::WalkerControl;
use rvt::ifc::gltf::model_to_glb;
use rvt::ifc::step_writer::{StepOptions, write_step_to};
use rvt::ifc::{ExportDiagnostics, ExportOptions, IfcModel};
use rvt::reader::UnreadStream;

use crate::args::{ExportGltfArgs, ExportIfcArgs, phase_of};
use crate::config::Failure;
use crate::output::{self, Written};
use crate::schema::{ExportOut, OutputOut};
use crate::store::{Entry, Store, note_texts, run_decode};
use crate::views::unread_out;

/// An export's summary, and the link to its file.
pub struct Exported {
    pub out: ExportOut,
    pub link: Resource,
}

/// The model an export is written from, and what its decode noted.
struct Source<'a> {
    model: &'a IfcModel,
    diagnostics: &'a ExportDiagnostics,
    unread: &'a [UnreadStream],
    notes: &'a [String],
}

/// The time `SOURCE_DATE_EPOCH` sets for the IFC header, or `None` when it is
/// unset or empty (reproducible-builds.org), as `rvt-ifc` reads it.
fn source_date_epoch() -> Result<Option<i64>, String> {
    match std::env::var("SOURCE_DATE_EPOCH") {
        Ok(value) if value.is_empty() => Ok(None),
        Ok(value) => value
            .parse()
            .map(Some)
            .map_err(|_| format!("SOURCE_DATE_EPOCH must be Unix seconds, not {value:?}")),
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(std::env::VarError::NotUnicode(value)) => Err(format!(
            "SOURCE_DATE_EPOCH must be Unix seconds, not {value:?}"
        )),
    }
}

/// The refusal `rvt-ifc` gives with exit status 3: a resource limit kept
/// streams from being read, and the caller did not accept an incomplete file.
fn refuse_partial(unread: &[UnreadStream], allow_partial: bool) -> Result<(), Failure> {
    if unread.is_empty() || allow_partial {
        return Ok(());
    }
    let mut message = String::from(
        "No file written: a resource limit kept part of the model from being read, so the export would be missing what these streams hold:",
    );
    for stream in unread {
        message.push_str(&format!("\n  {stream}"));
    }
    message.push_str(
        "\nStart the server with --large-model (or --max-stream-bytes and --max-inflate-bytes) to read them, or pass allow_partial=true to write the incomplete file.",
    );
    Err(Failure::Message(message))
}

fn finish(
    entry: &Entry,
    format: &str,
    mime_type: &str,
    written: &Written,
    source: &Source<'_>,
    with_diagnostics: bool,
) -> Exported {
    let diagnostics = source.diagnostics;
    let file_name = written.path.file_name().map_or_else(
        || "export".to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let link = Resource::new(output::file_uri(&written.path), file_name)
        .with_mime_type(mime_type)
        .with_size(written.bytes)
        .with_description(format!(
            "{format} export of {}; sha256 {}",
            entry.id, written.sha256
        ));
    let out = ExportOut {
        model_id: entry.id.clone(),
        format: format.to_string(),
        output: OutputOut {
            path: written.path.to_string_lossy().into_owned(),
            bytes: written.bytes,
            sha256: written.sha256.clone(),
        },
        building_elements: diagnostics.exported.building_elements,
        building_elements_with_geometry: diagnostics.exported.building_elements_with_geometry,
        entity_counts: diagnostics.exported.by_ifc_type.clone(),
        partial: !source.unread.is_empty(),
        unread_streams: unread_out(source.unread),
        notes: source.notes.to_vec(),
        diagnostics: if with_diagnostics {
            serde_json::to_value(diagnostics).ok()
        } else {
            None
        },
    };
    Exported { out, link }
}

pub fn ifc(
    store: &Store,
    args: &ExportIfcArgs,
    control: &WalkerControl,
) -> Result<Exported, Failure> {
    let entry = store.open(&args.model, phase_of(&args.phase))?;
    let config = store.config();
    let timestamp = source_date_epoch()?;
    let target = output::plan(
        config,
        &args.out_path,
        "ifc",
        args.overwrite,
        &entry.identity.path,
    )?;
    let switches = args.options.export_options();
    // The model kept for `rvt-ifc` with no switch is the export of the
    // default switches; any other switch decodes again on the same handle,
    // which still holds what the first decode inflated. Both are at the
    // phase the model was opened at.
    let default;
    let custom;
    let custom_notes;
    let source = if switches == ExportOptions::default() {
        default = entry.computed(config, control)?;
        Source {
            model: &default.model,
            diagnostics: &default.diagnostics,
            unread: &default.unread,
            notes: &default.notes,
        }
    } else {
        let options = switches.clone().with_phase(entry.phase.clone());
        custom = entry.with_file(|file| run_decode(file, config, &options, control))?;
        custom_notes = note_texts(&custom.notes);
        Source {
            model: &custom.model,
            diagnostics: &custom.diagnostics,
            unread: &custom.unread,
            notes: &custom_notes,
        }
    };
    refuse_partial(source.unread, args.options.allow_partial)?;
    switches
        .mode
        .validate(source.diagnostics)
        .map_err(|error| Failure::Message(error.to_string()))?;
    control.check().map_err(|_| Failure::Cancelled)?;
    let step_options = StepOptions { timestamp };
    let written = output::write(&target, args.overwrite, control, |writer| {
        write_step_to(source.model, &step_options, writer).map(|_| ())
    })?;
    Ok(finish(
        &entry,
        "ifc",
        "application/x-step",
        &written,
        &source,
        true,
    ))
}

pub fn gltf(
    store: &Store,
    args: &ExportGltfArgs,
    control: &WalkerControl,
) -> Result<Exported, Failure> {
    let entry = store.open(&args.model, phase_of(&args.phase))?;
    let config = store.config();
    let target = output::plan(
        config,
        &args.out_path,
        "glb",
        args.overwrite,
        &entry.identity.path,
    )?;
    let computed = entry.computed(config, control)?;
    refuse_partial(&computed.unread, args.allow_partial)?;
    control.check().map_err(|_| Failure::Cancelled)?;
    let glb = model_to_glb(&computed.model);
    let written = output::write(&target, args.overwrite, control, |writer| {
        writer.write_all(&glb)
    })?;
    let source = Source {
        model: &computed.model,
        diagnostics: &computed.diagnostics,
        unread: &computed.unread,
        notes: &computed.notes,
    };
    Ok(finish(
        &entry,
        "glb",
        "model/gltf-binary",
        &written,
        &source,
        false,
    ))
}
