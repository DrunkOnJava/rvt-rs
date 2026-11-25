//! Views, sheets and the viewports placed on them, from a project's native records.
//!
//! A Revit project stores every view as a `DBView*` object graph on channel
//! 102, and every view placed on a sheet as a `Viewport` record whose
//! `m_ownerDBViewId` names the sheet's view and whose `m_dbViewId` names the
//! view it shows (#1117). This module reads those records into a small,
//! serialisable catalog: the views (name, class, scale, plane and, for a 3D
//! view, its camera), the sheets (number, name and title-block fields) and
//! the placed viewports (their sheet, their view and their position on the
//! sheet).
//!
//! ## What is measured
//!
//! The viewport position rule is the one measured on #1117: `m_offset` is
//! where the view's origin lands on the sheet, so a viewport's box centre is
//! `m_offset + m_scale * crop centre`, where the crop box comes from the
//! view's `Viewer.m_boundedSpace.m_boundOffset`. One viewport of a
//! contributor's model was compared with `Viewport.GetBoxCenter()` from the
//! Revit API and agreed to the five decimals printed there.
//!
//! The 3D camera is read from `Viewer`: `m_boundedSpace.m_orig` is the eye
//! and `m_basis` the right, up and looking directions. [`Camera::from`]
//! accepts it only when the basis is orthonormal and, where the view stores
//! `m_horzDir`, `m_vertDir` and `m_viewDir`, agrees with them (the stored
//! looking direction is reversed). `m_targetPos` is reported only where it
//! lies on the looking axis; Revit stores a target off the axis on some 3D
//! views, where the eye and basis still describe the frame.
//!
//! A viewport turned on its sheet (`m_viewportOrientation` other than 0), a
//! view whose crop region is off, and a viewport whose crop box is not read
//! have no centre here: the turn and the uncropped extent are not measured.
//! Views are read on Revit 2015 to 2027's record framing; only 2023 to 2025
//! projects have measured element records, and the viewport comparison is
//! one viewport of one Revit 2024 project.

use crate::RevitFile;
use crate::native_document::{self, Options};
use anyhow::Result;
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Every view, sheet and placed viewport read from a file.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ViewCatalog {
    /// Views and sheets, one entry per `DBView*` record, ordered by id.
    pub views: Vec<View>,
    /// The views whose record carries a sheet number, ordered by id.
    pub sheets: Vec<Sheet>,
    /// `Viewport` records, placed or not, ordered by id.
    pub viewports: Vec<Viewport>,
}

/// What a `DBView*` record is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    /// A floor, ceiling, area or structural plan (`DBViewPlan`).
    Plan,
    /// A 3D view (`DBView3d`).
    ThreeD,
    /// A section or elevation (`DBViewSection`).
    Section,
    /// A drafting view (`DBViewDrafting` with no sheet number).
    Drafting,
    /// A sheet (`DBViewDrafting` with a sheet number).
    Sheet,
    /// A legend (`DBViewLegend`).
    Legend,
    /// A schedule (`DBViewSchedule`).
    Schedule,
    /// A `DBView*` class this reader does not name.
    Other,
}

/// One view's record.
#[derive(Debug, Clone, Serialize)]
pub struct View {
    /// The view's ElementId.
    pub id: u64,
    /// The record's class, as the file's schema spells it (`DBViewPlan`).
    pub class: String,
    pub kind: ViewKind,
    /// `m_viewName`.
    pub name: String,
    /// `m_viewDescription`.
    pub description: Option<String>,
    /// `m_scale`: the drawing scale's reciprocal (`0.01` is 1:100).
    pub scale: Option<f64>,
    /// `m_origin`, `m_horzDir`, `m_vertDir` and `m_viewDir`: the view plane's
    /// origin and its right, up and looking directions in model feet.
    pub plane: Option<ViewPlane>,
    /// `Viewer`: the view's crop box and, for a 3D view, its camera.
    pub viewer: Option<Viewer>,
    /// A plan view's `m_bottomClipElev`, `m_columnSymbolElev`,
    /// `m_cutPlaneElev`, `m_levelBelowElev` and `m_topClipElev`, in feet.
    pub view_range: Option<ViewRange>,
    /// `m_viewTemplateId`, where the view names one.
    pub template_id: Option<u64>,
    /// `m_isTemplate`.
    pub is_template: Option<bool>,
    /// A sheet's `m_sheetNumber`.
    pub sheet_number: Option<String>,
}

/// A view plane's origin and axes.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct ViewPlane {
    pub origin: [f64; 3],
    pub right: [f64; 3],
    pub up: [f64; 3],
    pub look: [f64; 3],
}

/// A plan view's range: bottom clip, column symbol, cut plane, view depth
/// and top clip, in feet.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ViewRange {
    pub bottom: f64,
    pub column_symbol: f64,
    pub cut_plane: f64,
    pub view_depth: f64,
    pub top: f64,
}

/// The `Viewer` record a view names (`m_viewerId`).
#[derive(Debug, Clone, Serialize)]
pub struct Viewer {
    /// The crop box's centre in the view's own coordinates, when the crop
    /// region is active.
    pub crop_center: Option<[f64; 2]>,
    /// The active crop extent, when one is read.
    pub crop: Option<CropBox>,
    /// `m_bReflected`: a ceiling plan looks up.
    pub reflected: Option<bool>,
    /// The 3D camera, when the viewer's fields describe one.
    pub camera: Option<Camera>,
}

/// A crop region's extent in view coordinates, in feet.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CropBox {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
    pub z_min: f64,
    pub z_max: f64,
}

impl CropBox {
    /// The crop's centre in the view's own x/y coordinates.
    pub fn center(&self) -> [f64; 2] {
        [
            (self.x_min + self.x_max) / 2.0,
            (self.y_min + self.y_max) / 2.0,
        ]
    }
}

/// A 3D view's camera: eye, target, up and the projection Revit stores.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Camera {
    pub eye: [f64; 3],
    /// `Viewer.m_targetPos` where it lies on the looking direction;
    /// `None` where the stored point is off that axis.
    pub target: Option<[f64; 3]>,
    pub up: [f64; 3],
    /// The looking direction (`m_basis`'s third row), which is the
    /// negative of the view plane's `m_viewDir`.
    pub look: [f64; 3],
    /// `Viewer.m_projMethodType`, as stored.
    pub projection_method: i64,
    /// `Viewer.m_projDist`, as stored.
    pub projection_distance: f64,
}

/// A sheet's title-block fields.
#[derive(Debug, Clone, Serialize)]
pub struct Sheet {
    /// The sheet view's ElementId.
    pub id: u64,
    /// `m_sheetNumber`.
    pub number: String,
    /// `m_viewName`.
    pub name: String,
    pub date: Option<String>,
    pub drawn_by: Option<String>,
    pub checked_by: Option<String>,
    pub designed_by: Option<String>,
    pub approved_by: Option<String>,
    pub issue_date: Option<String>,
    /// `m_bScheduled`: whether the sheet appears in a sheet list.
    pub scheduled: Option<bool>,
    /// `m_sheetViewportElemId` and `m_sheetViewportElemIds`.
    pub viewport_ids: Vec<u64>,
}

/// A `Viewport` record: a view's placement, on its sheet where it has one.
#[derive(Debug, Clone, Serialize)]
pub struct Viewport {
    /// The viewport's ElementId.
    pub id: u64,
    /// `m_ownerDBViewId`: the sheet's view. `None` for the internal viewport
    /// every view carries.
    pub sheet_id: Option<u64>,
    /// `m_dbViewId`: the view the viewport shows.
    pub view_id: Option<u64>,
    /// `m_detailNumber`; empty on a viewport with no detail number.
    pub detail_number: String,
    /// `m_offset`: where the view's origin lands on the sheet, in feet.
    pub offset: Option<[f64; 3]>,
    /// `m_scale`, the drawing scale's reciprocal.
    pub scale: Option<f64>,
    /// `m_viewportOrientation`.
    pub orientation: Option<i64>,
    /// `m_labelOrigin` and `m_labelLength`, as stored.
    pub label_origin: Option<[f64; 3]>,
    pub label_length: Option<f64>,
    /// The box centre on the sheet: `m_offset + m_scale * crop centre`,
    /// only where the viewport is not turned and its crop is active.
    pub center: Option<[f64; 2]>,
    /// The view's crop extent, when read.
    pub crop: Option<CropBox>,
    /// `ViewportAttributes.m_showLabel` of the viewport's type; `false`
    /// hides the view title.
    pub title_visible: Option<bool>,
    /// `ViewportAttributes.m_showExtensionLine` of the viewport's type.
    pub extension_line: Option<bool>,
}

impl ViewCatalog {
    /// Read every view, sheet and viewport in `file`; [`read`] with the
    /// catalog as the receiver.
    pub fn read(file: &mut RevitFile) -> Result<Self> {
        read(file)
    }

    /// Find a view by its ElementId.
    pub fn view(&self, id: u64) -> Option<&View> {
        self.views.iter().find(|view| view.id == id)
    }
    /// Find a sheet by its view's ElementId.
    pub fn sheet(&self, id: u64) -> Option<&Sheet> {
        self.sheets.iter().find(|sheet| sheet.id == id)
    }
    /// Every viewport placed on the sheet whose view has ElementId `sheet`.
    pub fn viewports_on(&self, sheet: u64) -> impl Iterator<Item = &Viewport> {
        self.viewports
            .iter()
            .filter(move |viewport| viewport.sheet_id == Some(sheet))
    }
}

/// Read every view, sheet and viewport in `file`.
///
/// The file is read through its native records (Revit 2015 to 2027), so a
/// file whose release is outside that range or whose record framing does not
/// decode returns an error rather than an empty catalog.
pub fn read(file: &mut RevitFile) -> Result<ViewCatalog> {
    let classes = native_document::class_names(file)?
        .into_iter()
        .filter(|class| {
            class.starts_with("DBView")
                || matches!(
                    class.as_str(),
                    "Viewport"
                        | "ViewportAttributes"
                        | "Viewer"
                        | "Viewer3d"
                        | "Section"
                        | "InteriorElevArrow"
                )
        })
        .collect::<BTreeSet<_>>();
    let options = Options {
        selected_classes: classes,
        releases: native_document::NATIVE_RELEASES,
        ..Options::default()
    };
    let mut records: BTreeMap<u64, (String, Value)> = BTreeMap::new();
    native_document::extract_graphs(file, &options, |record| {
        let Some(graph) = record.graph.as_ref() else {
            return Ok(());
        };
        let Some(root) = graph.objects.first() else {
            return Ok(());
        };
        if let Some(class) = record.class_name {
            records.insert(record.identity.element_id, (class, root.fields.to_json()));
        }
        Ok(())
    })?;

    let mut views = Vec::new();
    let mut sheets = Vec::new();
    let mut viewports = Vec::new();
    let mut viewport_types = BTreeMap::new();
    let mut viewers: BTreeMap<u64, Value> = BTreeMap::new();

    for (id, (class, fields)) in &records {
        match class.as_str() {
            "ViewportAttributes" => {
                viewport_types.insert(
                    *id,
                    (
                        fields.get("m_showLabel").and_then(Value::as_i64),
                        fields.get("m_showExtensionLine").and_then(Value::as_bool),
                    ),
                );
            }
            "Viewer" | "Viewer3d" | "Section" | "InteriorElevArrow" => {
                viewers.insert(*id, fields.clone());
            }
            _ if is_view_class(class) => {
                if let Some(view) = view(*id, class, fields, &viewers) {
                    if let Some(number) = view.sheet_number.clone() {
                        sheets.push(sheet(*id, &number, fields));
                    }
                    views.push(view);
                }
            }
            _ => {}
        }
    }

    for (id, (class, fields)) in &records {
        if class != "Viewport" {
            continue;
        }
        let sheet_id = id_of(fields, "m_ownerDBViewId");
        let view_id = id_of(fields, "m_dbViewId");
        let offset = vector(fields.get("m_offset"));
        let scale = fields.get("m_scale").and_then(Value::as_f64);
        let orientation = fields.get("m_viewportOrientation").and_then(Value::as_i64);
        let crop = view_id
            .and_then(|view| records.get(&view))
            .and_then(|(_, view)| {
                id_of(view, "m_viewerId")
                    .and_then(|viewer_id| viewers.get(&viewer_id))
                    .map(|viewer_fields| viewer(viewer_fields, view, false))
            })
            .and_then(|viewer| viewer.crop);
        let center = match (sheet_id, orientation, offset, scale, crop) {
            (Some(_), Some(0), Some(offset), Some(scale), Some(crop)) => {
                let center = crop.center();
                Some([offset[0] + scale * center[0], offset[1] + scale * center[1]])
            }
            _ => None,
        };
        let (title_visible, extension_line) = id_of(fields, "m_attrId")
            .and_then(|kind| viewport_types.get(&kind))
            .map_or((None, None), |(label, line)| {
                (label.map(|label| label != 0), *line)
            });
        viewports.push(Viewport {
            id: *id,
            sheet_id,
            view_id,
            detail_number: string(fields, "m_detailNumber").unwrap_or_default(),
            offset,
            scale,
            orientation,
            label_origin: vector(fields.get("m_labelOrigin")),
            label_length: fields.get("m_labelLength").and_then(Value::as_f64),
            center,
            crop,
            title_visible,
            extension_line,
        });
    }

    views.sort_by_key(|view| view.id);
    sheets.sort_by_key(|sheet| sheet.id);
    viewports.sort_by_key(|viewport| viewport.id);
    Ok(ViewCatalog {
        views,
        sheets,
        viewports,
    })
}

fn view(id: u64, class: &str, fields: &Value, viewers: &BTreeMap<u64, Value>) -> Option<View> {
    let viewer_id = id_of(fields, "m_viewerId");
    let viewer = viewer_id
        .and_then(|viewer_id| viewers.get(&viewer_id))
        .map(|viewer_fields| viewer(viewer_fields, fields, class == "DBView3d"));
    let sheet_number = string(fields, "m_sheetNumber");
    let kind = match class {
        "DBViewPlan" => ViewKind::Plan,
        "DBView3d" => ViewKind::ThreeD,
        "DBViewSection" => ViewKind::Section,
        "DBViewDrafting" | "DBViewAnonDrafting" if sheet_number.is_none() => ViewKind::Drafting,
        "DBViewDrafting" | "DBViewAnonDrafting" => ViewKind::Sheet,
        "DBViewLegend" => ViewKind::Legend,
        "DBViewSchedule" => ViewKind::Schedule,
        _ => ViewKind::Other,
    };
    Some(View {
        id,
        class: class.to_string(),
        kind,
        name: string(fields, "m_viewName").unwrap_or_default(),
        description: string(fields, "m_viewDescription"),
        scale: fields.get("m_scale").and_then(Value::as_f64),
        plane: plane(fields),
        viewer,
        view_range: view_range(fields),
        template_id: id_of(fields, "m_viewTemplateId"),
        is_template: fields.get("m_isTemplate").and_then(Value::as_bool),
        sheet_number,
    })
}

fn sheet(id: u64, number: &str, fields: &Value) -> Sheet {
    Sheet {
        id,
        number: number.to_string(),
        name: string(fields, "m_viewName").unwrap_or_default(),
        date: string(fields, "m_date"),
        drawn_by: string(fields, "m_drawnBy"),
        checked_by: string(fields, "m_checkedBy"),
        designed_by: string(fields, "m_designedBy"),
        approved_by: string(fields, "m_approvedBy"),
        issue_date: string(fields, "m_issueDate"),
        scheduled: fields.get("m_bScheduled").and_then(Value::as_bool),
        viewport_ids: {
            let mut ids = Vec::new();
            if let Some(id) = id_of(fields, "m_sheetViewportElemId") {
                ids.push(id);
            }
            if let Some(items) = fields
                .get("m_sheetViewportElemIds")
                .and_then(Value::as_array)
            {
                ids.extend(items.iter().filter_map(id_from_value));
            }
            ids.sort_unstable();
            ids.dedup();
            ids
        },
    }
}

fn viewer(fields: &Value, view: &Value, with_camera: bool) -> Viewer {
    let bounded = fields.get("m_boundedSpace");
    let crop = bounded.and_then(|bounded| {
        let active = bounded.get("m_boundActive")?;
        let box_ = bounded.get("m_boundOffset")?;
        crop_box(box_, active, fields.get("m_viewerFlags"))
    });
    Viewer {
        crop_center: crop.map(|crop| crop.center()),
        crop,
        reflected: fields.get("m_bReflected").and_then(Value::as_bool),
        camera: Camera::from(fields, view).filter(|_| with_camera),
    }
}

impl Camera {
    /// Read a 3D camera from a `Viewer` record, only where its fields agree:
    /// an orthonormal basis that is the view plane's own right, up and
    /// reversed looking directions (where the view stores them), and a
    /// target on the looking axis where the stored `m_targetPos` is one.
    ///
    /// Revit keeps the target off the axis on some 3D views (an
    /// orthographic view's target can be anywhere in its plane); the
    /// camera's eye, up and look are still the view's, so the target is
    /// reported only where it is on the axis.
    pub fn from(fields: &Value, view: &Value) -> Option<Self> {
        let bounded = fields.get("m_boundedSpace")?;
        let eye = vector(bounded.get("m_orig"))?;
        let basis = matrix(bounded.get("m_basis"))?;
        let right = basis[0];
        let up = basis[1];
        let look = basis[2];
        let len = |v: [f64; 3]| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        for axis in [right, up, look] {
            let length = len(axis);
            if !(0.99..=1.01).contains(&length) {
                return None;
            }
        }
        for (a, b) in [(right, up), (right, look), (up, look)] {
            if dot(a, b).abs() > 1e-4 {
                return None;
            }
        }
        // The view plane stores the same basis, with its looking direction
        // reversed. Where the file has both, they must be one basis.
        if let (Some(right_), Some(up_), Some(look_)) = (
            vector(view.get("m_horzDir")),
            vector(view.get("m_vertDir")),
            vector(view.get("m_viewDir")),
        ) {
            if dot(right, right_) < 1.0 - 1e-4
                || dot(up, up_) < 1.0 - 1e-4
                || dot(look, [-look_[0], -look_[1], -look_[2]]) < 1.0 - 1e-4
            {
                return None;
            }
        }
        let target = vector(fields.get("m_targetPos")).and_then(|target| {
            let delta = [target[0] - eye[0], target[1] - eye[1], target[2] - eye[2]];
            let distance = len(delta);
            (distance > 0.0
                && dot(
                    [
                        delta[0] / distance,
                        delta[1] / distance,
                        delta[2] / distance,
                    ],
                    look,
                ) >= 1.0 - 1e-4)
                .then_some(target)
        });
        Some(Self {
            eye,
            target,
            up,
            look,
            projection_method: fields.get("m_projMethodType").and_then(Value::as_i64)?,
            projection_distance: fields.get("m_projDist").and_then(Value::as_f64)?,
        })
    }
}

fn crop_box(box_: &Value, active: &Value, flags: Option<&Value>) -> Option<CropBox> {
    // `m_viewerFlags` bit 0 says the crop region is on; `m_boundActive` says
    // which of the box's six bounds bound the view.
    if flags
        .and_then(Value::as_u64)
        .is_some_and(|flags| flags & 1 == 0)
    {
        return None;
    }
    let ranges = rows(box_)?;
    let active = rows_u8(active)?;
    if ranges.len() != 3 || active.len() != 3 {
        return None;
    }
    let mut extent = [[0.0; 2]; 3];
    let mut any = false;
    for (axis, (range, on)) in ranges.iter().zip(&active).enumerate() {
        if range.len() != 2 || on.len() != 2 {
            return None;
        }
        let low = range[0].min(range[1]);
        let high = range[0].max(range[1]);
        extent[axis] = [low, high];
        any |= on.iter().any(|on| *on != 0.0);
    }
    if !any {
        return None;
    }
    Some(CropBox {
        x_min: extent[0][0],
        x_max: extent[0][1],
        y_min: extent[1][0],
        y_max: extent[1][1],
        z_min: extent[2][0],
        z_max: extent[2][1],
    })
}

fn plane(fields: &Value) -> Option<ViewPlane> {
    Some(ViewPlane {
        origin: vector(fields.get("m_origin"))?,
        right: vector(fields.get("m_horzDir"))?,
        up: vector(fields.get("m_vertDir"))?,
        look: vector(fields.get("m_viewDir"))?,
    })
}

fn view_range(fields: &Value) -> Option<ViewRange> {
    Some(ViewRange {
        bottom: fields.get("m_bottomClipElev")?.as_f64()?,
        column_symbol: fields.get("m_columnSymbolElev")?.as_f64()?,
        cut_plane: fields.get("m_cutPlaneElev")?.as_f64()?,
        view_depth: fields.get("m_levelBelowElev")?.as_f64()?,
        top: fields.get("m_topClipElev")?.as_f64()?,
    })
}

/// Whether a `DBView*` class is a user view rather than a type, a preview,
/// a project-browser root or a cutter helper.
fn is_view_class(class: &str) -> bool {
    class.starts_with("DBView")
        && !matches!(
            class,
            "DBViewProject"
                | "DBViewInfo"
                | "DBViewInfoForPreview"
                | "DBView3dEnergyAnalysisPreview"
                | "DBViewScheduleData"
                | "DBViewType"
                | "DBViewTypesForNewLevel"
                | "DBViewPlanCutter"
                | "DBViewPlanClosedCutter"
                | "DBViewSectionCutter"
                | "DBViewSectionCutterFCI"
                | "DBView3dCutter"
        )
        && !class.ends_with("Cutter")
}

fn id_of(fields: &Value, member: &str) -> Option<u64> {
    id_from_value(fields.get(member)?)
}

fn id_from_value(value: &Value) -> Option<u64> {
    let id = value.get("m_id")?.get("m_id64")?.as_i64()?;
    u64::try_from(id).ok()
}

fn string(fields: &Value, member: &str) -> Option<String> {
    fields
        .get(member)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
}

fn vector(value: Option<&Value>) -> Option<[f64; 3]> {
    let items = value?.as_array()?;
    if items.len() != 3 {
        return None;
    }
    Some([items[0].as_f64()?, items[1].as_f64()?, items[2].as_f64()?])
}

fn matrix(value: Option<&Value>) -> Option<[[f64; 3]; 3]> {
    let rows = value?.as_array()?;
    if rows.len() != 3 {
        return None;
    }
    Some([
        vector(Some(&rows[0]))?,
        vector(Some(&rows[1]))?,
        vector(Some(&rows[2]))?,
    ])
}

fn rows(value: &Value) -> Option<Vec<Vec<f64>>> {
    value
        .as_array()?
        .iter()
        .map(|row| row.as_array()?.iter().map(Value::as_f64).collect())
        .collect()
}

fn rows_u8(value: &Value) -> Option<Vec<Vec<f64>>> {
    value
        .as_array()?
        .iter()
        .map(|row| {
            row.as_array()?
                .iter()
                .map(|item| match item {
                    Value::Bool(flag) => Some(f64::from(u8::from(*flag))),
                    value => value.as_f64(),
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crop_box_needs_an_active_bound() {
        let bounds = serde_json::json!([[10.0, -10.0], [4.0, 2.0], [0.0, 1.0]]);
        let inactive = serde_json::json!([[false, false], [false, false], [false, false]]);
        let active = serde_json::json!([[true, true], [true, false], [false, false]]);
        assert!(crop_box(&bounds, &inactive, Some(&serde_json::json!(1))).is_none());
        let crop = crop_box(&bounds, &active, Some(&serde_json::json!(1))).unwrap();
        assert_eq!((crop.x_min, crop.x_max), (-10.0, 10.0));
        assert_eq!((crop.y_min, crop.y_max), (2.0, 4.0));
        assert!(crop_box(&bounds, &active, Some(&serde_json::json!(0))).is_none());
    }

    #[test]
    fn camera_needs_an_orthonormal_basis_pointing_at_the_target() {
        let view = serde_json::json!({
            "m_horzDir": [1.0, 0.0, 0.0],
            "m_vertDir": [0.0, 1.0, 0.0],
            "m_viewDir": [0.0, 0.0, 1.0]
        });
        let fields = serde_json::json!({
            "m_boundedSpace": {
                "m_orig": [0.0, 0.0, 10.0],
                "m_basis": [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]]
            },
            "m_targetPos": [0.0, 0.0, 0.0],
            "m_projMethodType": 1,
            "m_projDist": 0.1
        });
        let camera = Camera::from(&fields, &view).expect("camera");
        assert_eq!(camera.target, Some([0.0, 0.0, 0.0]));
        // A target off the axis is not reported, but the camera's basis is.
        let mut wrong = fields.clone();
        wrong["m_targetPos"] = serde_json::json!([1.0, 0.0, 0.0]);
        assert_eq!(Camera::from(&wrong, &view).expect("camera").target, None);
        // A skewed basis is not a camera.
        let mut skewed = fields;
        skewed["m_boundedSpace"]["m_basis"][1] = serde_json::json!([0.6, 0.8, 0.0]);
        assert!(Camera::from(&skewed, &view).is_none());
        // The view plane and the viewer must agree.
        let inconsistent = serde_json::json!({
            "m_horzDir": [0.0, 1.0, 0.0],
            "m_vertDir": [0.0, 0.0, 1.0],
            "m_viewDir": [1.0, 0.0, 0.0]
        });
        assert!(Camera::from(&skewed, &inconsistent).is_none());
    }
}
