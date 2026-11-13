//! What the tools take. Each type's schema is the tool's `inputSchema`.

use rvt::ifc::{ExportOptions, ExportQualityMode, SavedMeshChoice};
use schemars::JsonSchema;
use serde::Deserialize;

/// The longest phase name the server takes. It keeps every phase it is asked
/// for (so that asking again does not read the file again), and no phase of a
/// Revit project has a name near this long.
pub const MAX_PHASE_CHARS: usize = 256;

/// The phase an argument names: none when it is left out or blank.
pub fn phase_of(phase: &Option<String>) -> Option<&str> {
    phase
        .as_deref()
        .map(str::trim)
        .filter(|phase| !phase.is_empty())
}

/// A model, by the `model_id` `open_model` returned or by path.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct ModelArgs {
    /// The `model_id` that `open_model` returned, or the path of a `.rvt`,
    /// `.rfa`, `.rte` or `.rft` file inside the server's root folder (the
    /// folder it was started in, or `--root`); a relative path is taken from
    /// that folder, and a path outside it is refused. A path that is not open
    /// yet is opened.
    #[serde(alias = "model_id", alias = "path")]
    pub model: String,
    /// The project phase to read, by its name (exactly) or its ElementId,
    /// instead of the project's last (`rvt-ifc --phase`). A phase named is
    /// exact: naming the last phase differs from leaving `phase` out where the
    /// last phase holds no room, since leaving it out then reads the rooms of
    /// the latest phase that holds any. It applies to a file
    /// path: a `model_id` names the phase its model was opened at. An unknown
    /// phase is an error that lists the file's phases.
    pub phase: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListElementsArgs {
    /// The `model_id` that `open_model` returned, or the path of the file.
    #[serde(alias = "model_id", alias = "path")]
    pub model: String,
    /// The project phase to read, by its name (exactly) or its ElementId,
    /// instead of the project's last (`rvt-ifc --phase`). A phase named is
    /// exact: naming the last phase differs from leaving `phase` out where the
    /// last phase holds no room, since leaving it out then reads the rooms of
    /// the latest phase that holds any. It applies to a file
    /// path: a `model_id` names the phase its model was opened at. An unknown
    /// phase is an error that lists the file's phases.
    pub phase: Option<String>,
    /// Only elements of this IFC entity: `IFCWALL`, `IfcWall` and `wall` all
    /// name walls. `get_summary` lists the entities the model holds.
    pub category: Option<String>,
    /// Only elements of this Revit class, such as `Wall`, `Room` or `Level`,
    /// as the element's record names it (`provenance.class`).
    pub class: Option<String>,
    /// Only elements on the storey with this name (see `list_levels`).
    pub level: Option<String>,
    /// How many elements to return: 100 by default, from 1 to 1000. A value
    /// outside that range is an error.
    #[schemars(range(min = 1, max = 1000))]
    pub limit: Option<u32>,
    /// How many matching elements to skip, for the next page.
    pub offset: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListRoomsArgs {
    /// The `model_id` that `open_model` returned, or the path of the file.
    #[serde(alias = "model_id", alias = "path")]
    pub model: String,
    /// The project phase to read, by its name (exactly) or its ElementId,
    /// instead of the project's last (`rvt-ifc --phase`). A phase named is
    /// exact: naming the last phase differs from leaving `phase` out where the
    /// last phase holds no room, since leaving it out then reads the rooms of
    /// the latest phase that holds any. It applies to a file
    /// path: a `model_id` names the phase its model was opened at. An unknown
    /// phase is an error that lists the file's phases.
    pub phase: Option<String>,
    /// How many spaces to return: 1000 by default, from 1 to 1000. A value
    /// outside that range is an error. The answer's `total` says how many
    /// there are; when `returned` is less, call again with `offset`.
    #[schemars(range(min = 1, max = 1000))]
    pub limit: Option<u32>,
    /// How many spaces to skip, for the next page.
    pub offset: Option<u32>,
}

/// An ElementId or a GlobalId.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum IdArg {
    Number(i64),
    Text(String),
}

impl IdArg {
    pub fn text(&self) -> String {
        match self {
            Self::Number(number) => number.to_string(),
            Self::Text(text) => text.trim().to_string(),
        }
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetElementArgs {
    /// The `model_id` that `open_model` returned, or the path of the file.
    #[serde(alias = "model_id", alias = "path")]
    pub model: String,
    /// The element's Revit ElementId (`1234`) or its 22-character IFC
    /// GlobalId, as `list_elements` gives them.
    pub id: IdArg,
    /// The project phase to read, by its name (exactly) or its ElementId,
    /// instead of the project's last (`rvt-ifc --phase`). A phase named is
    /// exact: naming the last phase differs from leaving `phase` out where the
    /// last phase holds no room, since leaving it out then reads the rooms of
    /// the latest phase that holds any. It applies to a file
    /// path: a `model_id` names the phase its model was opened at. An unknown
    /// phase is an error that lists the file's phases.
    pub phase: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExplainElementArgs {
    /// The `model_id` that `open_model` returned, or the path of the file.
    #[serde(alias = "model_id", alias = "path")]
    pub model: String,
    /// The element's Revit ElementId (`1234`), whether or not the export holds
    /// it, or the 22-character IFC GlobalId of an exported one.
    #[serde(alias = "id")]
    pub element: IdArg,
    /// The project phase to read, by its name (exactly) or its ElementId,
    /// instead of the project's last (`rvt-ifc --phase`). A phase named is
    /// exact: naming the last phase differs from leaving `phase` out where the
    /// last phase holds no room, since leaving it out then reads the rooms of
    /// the latest phase that holds any. It applies to a file
    /// path: a `model_id` names the phase its model was opened at. An unknown
    /// phase is an error that lists the file's phases.
    pub phase: Option<String>,
}

/// The quality the export must reach, as `rvt-ifc --mode`.
#[derive(Debug, Default, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
pub enum ModeArg {
    /// Write what is recovered, however little (the default).
    #[default]
    Scaffold,
    /// Require validated typed elements, and write no geometry.
    TypedNoGeometry,
    /// Require at least one element with geometry.
    Geometry,
    /// Also require project metadata, units, levels and no warnings.
    Strict,
}

impl From<ModeArg> for ExportQualityMode {
    fn from(mode: ModeArg) -> Self {
        match mode {
            ModeArg::Scaffold => Self::Scaffold,
            ModeArg::TypedNoGeometry => Self::TypedNoGeometry,
            ModeArg::Geometry => Self::Geometry,
            ModeArg::Strict => Self::Strict,
        }
    }
}

/// Which bodies are drawn from the mesh Revit saved for display.
#[derive(Debug, Default, Clone, Copy, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MeshesArg {
    /// Every element that would be its record's bounding box, on Revit 2023 to
    /// 2027 (the default).
    #[default]
    Default,
    /// Doors and windows only, on Revit 2023 to 2027: the default before #1191,
    /// and cheaper in time and memory.
    DoorsAndWindows,
    /// Every element that would be its record's bounding box, on every release.
    All,
    /// None: every element keeps its record's box.
    Off,
}

/// The `rvt-ifc` switches that change the model. The defaults are `rvt-ifc`'s.
#[derive(Debug, Default, Deserialize, JsonSchema)]
pub struct IfcOptions {
    /// Required quality (`rvt-ifc --mode`).
    #[serde(default)]
    pub mode: ModeArg,
    /// Bodies drawn from saved meshes (`rvt-ifc --saved-meshes`,
    /// `--no-saved-meshes`).
    #[serde(default)]
    pub saved_meshes: MeshesArg,
    /// Also write Revit's own parameter sets (`--revit-property-sets`).
    #[serde(default)]
    pub revit_property_sets: bool,
    /// Also place properties where Revit's exporter does, and keep the
    /// material it associates with an `IfcSpace`, which IFC4 does not allow
    /// (`--revit-set-placement`).
    #[serde(default)]
    pub revit_set_placement: bool,
    /// Leave out the base quantities (`--no-base-quantities`).
    #[serde(default)]
    pub no_base_quantities: bool,
    /// Write the file even when a resource limit kept part of the model from
    /// being read (`--allow-partial`). The result lists the unread streams.
    #[serde(default)]
    pub allow_partial: bool,
}

impl IfcOptions {
    /// The export these switches ask for. The phase is not among them: a
    /// model is opened at its phase.
    pub fn export_options(&self) -> ExportOptions {
        ExportOptions::default()
            .with_mode(self.mode.into())
            .with_saved_meshes(match self.saved_meshes {
                MeshesArg::Default => SavedMeshChoice::Default,
                MeshesArg::DoorsAndWindows => SavedMeshChoice::DoorsAndWindows,
                MeshesArg::All => SavedMeshChoice::All,
                MeshesArg::Off => SavedMeshChoice::Off,
            })
            .with_revit_property_sets(self.revit_property_sets)
            .with_revit_set_placement(self.revit_set_placement)
            .with_no_base_quantities(self.no_base_quantities)
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportIfcArgs {
    /// The `model_id` that `open_model` returned, or the path of the file.
    #[serde(alias = "model_id", alias = "path")]
    pub model: String,
    /// Where to write the IFC4 file, inside the server's root folder (or its
    /// `--output-dir`); a relative path is taken from that folder. It must end
    /// in `.ifc`, must not be the model, and must not exist unless `overwrite`
    /// is true.
    pub out_path: String,
    /// Replace `out_path` if it exists.
    #[serde(default)]
    pub overwrite: bool,
    /// The `rvt-ifc` switches; none by default.
    #[serde(default)]
    pub options: IfcOptions,
    /// The project phase to read, by its name (exactly) or its ElementId,
    /// instead of the project's last (`rvt-ifc --phase`). A phase named is
    /// exact: naming the last phase differs from leaving `phase` out where the
    /// last phase holds no room, since leaving it out then reads the rooms of
    /// the latest phase that holds any. It applies to a file
    /// path: a `model_id` names the phase its model was opened at. An unknown
    /// phase is an error that lists the file's phases.
    pub phase: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExportGltfArgs {
    /// The `model_id` that `open_model` returned, or the path of the file.
    #[serde(alias = "model_id", alias = "path")]
    pub model: String,
    /// Where to write the binary glTF, inside the server's root folder (or its
    /// `--output-dir`); a relative path is taken from that folder. It must end
    /// in `.glb`, must not be the model, and must not exist unless `overwrite`
    /// is true.
    pub out_path: String,
    /// Replace `out_path` if it exists.
    #[serde(default)]
    pub overwrite: bool,
    /// Write the file even when a resource limit kept part of the model from
    /// being read. The result lists the unread streams.
    #[serde(default)]
    pub allow_partial: bool,
    /// The project phase to read, by its name (exactly) or its ElementId,
    /// instead of the project's last (`rvt-ifc --phase`). A phase named is
    /// exact: naming the last phase differs from leaving `phase` out where the
    /// last phase holds no room, since leaving it out then reads the rooms of
    /// the latest phase that holds any. It applies to a file
    /// path: a `model_id` names the phase its model was opened at. An unknown
    /// phase is an error that lists the file's phases.
    pub phase: Option<String>,
}
