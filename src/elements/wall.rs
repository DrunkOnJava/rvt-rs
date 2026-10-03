//! `Wall` + `WallType` — the single highest-leverage element class in
//! any architectural Revit model. Decoded here into stable typed
//! views; geometry assembly (extrusion from location curve + layered
//! compound structure) lives in `src/geometry/` and is task GEO-27.
//!
//! # Typical Revit field shape (names stable 2016–2026)
//!
//! Wall:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_level_id` | ElementId | Base level (from `Level` decoder) |
//! | `m_base_offset` | f64 | Height above base level (feet) |
//! | `m_top_level_id` | ElementId | Top level, or 0 if "unconnected" |
//! | `m_top_offset` | f64 | Height offset above top level |
//! | `m_unconnected_height` | f64 | Used when `m_top_level_id == 0` |
//! | `m_structural_usage` | Primitive u32 | 0=NonBearing 1=Bearing 2=Shear 3=Combined |
//! | `m_orientation` | Primitive u32 | 0=Interior 1=Exterior |
//! | `m_location_line` | Primitive u32 | 0=Centerline 1=Core-Centerline 2=Finish-Exterior … |
//! | `m_type_id` | ElementId | Reference to `WallType` |
//! | `m_host_id` | ElementId | Host element, 0 = model-hosted |
//!
//! The 2D `location curve` (the line in plan view that the wall was
//! drawn along) is stored by the ADocument walker separately — the
//! `location_curve_id` here is a handle into that table. Wiring that
//! up is task L5B-01 + GEO-27.
//!
//! WallType:
//!
//! | Field | Type | Semantics |
//! |---|---|---|
//! | `m_name` | String | "Generic - 8\"", "Basic Wall", … |
//! | `m_kind` | Primitive u32 | 0=Basic 1=Curtain 2=Stacked |
//! | `m_function` | Primitive u32 | 0=Interior 1=Exterior 2=Foundation 3=Retaining 4=Soffit 5=CoreShaft |
//! | `m_width` | f64 | Total wall thickness in feet (sum of layers) |
//! | `m_structural` | Primitive bool | Load-bearing by default? |
//!
//! Multi-layer compound structure (`m_compound`) is a nested structure
//! that the walker's Vector variant (L5B-08) will surface later. For
//! now we capture the scalar fields.

use super::level::normalise_field_name;
use crate::formats;
use crate::geometry::Point3;
use crate::walker::{DecodedElement, ElementDecoder, HandleIndex, InstanceField};
use crate::{Error, Result};

/// How a wall's load-bearing role is classified.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StructuralUsage {
    #[default]
    NonBearing,
    Bearing,
    Shear,
    Combined,
}

impl StructuralUsage {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Bearing,
            2 => Self::Shear,
            3 => Self::Combined,
            _ => Self::NonBearing,
        }
    }
}

/// Which side of the location line represents the "face" Revit shows
/// in outlines and which side receives finish layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocationLine {
    #[default]
    WallCenterline,
    CoreCenterline,
    FinishFaceExterior,
    FinishFaceInterior,
    CoreFaceExterior,
    CoreFaceInterior,
}

impl LocationLine {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::CoreCenterline,
            2 => Self::FinishFaceExterior,
            3 => Self::FinishFaceInterior,
            4 => Self::CoreFaceExterior,
            5 => Self::CoreFaceInterior,
            _ => Self::WallCenterline,
        }
    }
}

/// The three wall classifications Revit stores at the type level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallKind {
    #[default]
    Basic,
    Curtain,
    Stacked,
}

impl WallKind {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Curtain,
            2 => Self::Stacked,
            _ => Self::Basic,
        }
    }

    /// `true` when the wall is a plain Basic wall (most common case).
    pub fn is_basic(self) -> bool {
        matches!(self, Self::Basic)
    }
}

/// The functional role a wall plays architecturally — drives IFC
/// predefined-type tagging (`INTERNAL`, `PARTITIONING`, `PARAPET`…).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallFunction {
    #[default]
    Interior,
    Exterior,
    Foundation,
    Retaining,
    Soffit,
    CoreShaft,
}

impl WallFunction {
    pub fn from_code(code: u32) -> Self {
        match code {
            1 => Self::Exterior,
            2 => Self::Foundation,
            3 => Self::Retaining,
            4 => Self::Soffit,
            5 => Self::CoreShaft,
            _ => Self::Interior,
        }
    }

    /// Map to the closest IFC `IfcWallTypeEnum` predefined type.
    pub fn to_ifc_predefined(self) -> &'static str {
        match self {
            Self::Interior => "PARTITIONING",
            Self::Exterior => "STANDARD",
            Self::Foundation => "ELEMENTEDWALL",
            Self::Retaining => "SOLIDWALL",
            Self::Soffit => "PARAPET",
            Self::CoreShaft => "SHEAR",
        }
    }
}

/// Registered decoder for the `Wall` class.
pub struct WallDecoder;

impl ElementDecoder for WallDecoder {
    fn class_name(&self) -> &'static str {
        "Wall"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "Wall" {
            return Err(Error::BasicFileInfo(format!(
                "WallDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Registered decoder for the `WallType` class.
pub struct WallTypeDecoder;

impl ElementDecoder for WallTypeDecoder {
    fn class_name(&self) -> &'static str {
        "WallType"
    }

    fn decode(
        &self,
        bytes: &[u8],
        schema: &formats::ClassEntry,
        _index: &HandleIndex,
    ) -> Result<DecodedElement> {
        if schema.name != "WallType" {
            return Err(Error::BasicFileInfo(format!(
                "WallTypeDecoder received wrong schema: {}",
                schema.name
            )));
        }
        Ok(crate::walker::decode_instance(bytes, 0, schema))
    }
}

/// Typed view of a decoded Wall instance.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Wall {
    pub base_level_id: Option<u32>,
    pub base_offset_feet: Option<f64>,
    /// `0` or `None` means the wall is "unconnected" at the top.
    pub top_level_id: Option<u32>,
    pub top_offset_feet: Option<f64>,
    pub unconnected_height_feet: Option<f64>,
    pub structural_usage: Option<StructuralUsage>,
    pub location_line: Option<LocationLine>,
    pub type_id: Option<u32>,
    pub host_id: Option<u32>,
    /// Handle into the document location-curve table when present.
    /// Geometry resolution is [`crate::geometry::recovery`].
    pub location_curve_id: Option<u32>,
    /// Explicit location-curve start when schema fields carry XYZ/XY.
    pub location_start: Option<Point3>,
    /// Explicit location-curve end when schema fields carry XYZ/XY.
    pub location_end: Option<Point3>,
}

impl Wall {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        let mut sx = None;
        let mut sy = None;
        let mut sz = None;
        let mut ex = None;
        let mut ey = None;
        let mut ez = None;
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("levelid" | "baselevelid", InstanceField::ElementId { id, .. }) => {
                    out.base_level_id = Some(*id);
                }
                ("baseoffset", InstanceField::Float { value, .. }) => {
                    out.base_offset_feet = Some(*value);
                }
                ("toplevelid", InstanceField::ElementId { id, .. }) => {
                    out.top_level_id = Some(*id);
                }
                ("topoffset", InstanceField::Float { value, .. }) => {
                    out.top_offset_feet = Some(*value);
                }
                // gen-fixture Walls carry `m_height`; real files use
                // `m_unconnected_height` when top is unbound.
                ("unconnectedheight" | "height", InstanceField::Float { value, .. }) => {
                    out.unconnected_height_feet = Some(*value);
                }
                ("structuralusage", InstanceField::Integer { value, .. }) => {
                    out.structural_usage = Some(StructuralUsage::from_code(*value as u32));
                }
                ("locationline", InstanceField::Integer { value, .. }) => {
                    out.location_line = Some(LocationLine::from_code(*value as u32));
                }
                ("typeid", InstanceField::ElementId { id, .. }) => out.type_id = Some(*id),
                ("hostid", InstanceField::ElementId { id, .. }) => out.host_id = Some(*id),
                (
                    "locationcurveid" | "locationcurve" | "curvelineid" | "curveline",
                    InstanceField::ElementId { id, .. },
                ) => {
                    if *id != 0 {
                        out.location_curve_id = Some(*id);
                    }
                }
                (
                    "startx" | "locationstartx" | "curvestartx",
                    InstanceField::Float { value, .. },
                ) => {
                    sx = Some(*value);
                }
                (
                    "starty" | "locationstarty" | "curvestarty",
                    InstanceField::Float { value, .. },
                ) => {
                    sy = Some(*value);
                }
                (
                    "startz" | "locationstartz" | "curvestartz",
                    InstanceField::Float { value, .. },
                ) => {
                    sz = Some(*value);
                }
                ("endx" | "locationendx" | "curveendx", InstanceField::Float { value, .. }) => {
                    ex = Some(*value);
                }
                ("endy" | "locationendy" | "curveendy", InstanceField::Float { value, .. }) => {
                    ey = Some(*value);
                }
                ("endz" | "locationendz" | "curveendz", InstanceField::Float { value, .. }) => {
                    ez = Some(*value);
                }
                _ => {}
            }
        }
        if let (Some(x), Some(y)) = (sx, sy) {
            out.location_start = Some(Point3::new(x, y, sz.unwrap_or(0.0)));
        }
        if let (Some(x), Some(y)) = (ex, ey) {
            out.location_end = Some(Point3::new(x, y, ez.unwrap_or(0.0)));
        }
        out
    }

    /// `true` when the wall's top isn't bound to a level — in that case
    /// height comes from `unconnected_height_feet`.
    pub fn is_unconnected(&self) -> bool {
        matches!(self.top_level_id, None | Some(0))
    }
}

/// Typed view of a decoded WallType.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WallType {
    pub name: Option<String>,
    pub kind: Option<WallKind>,
    pub function: Option<WallFunction>,
    pub width_feet: Option<f64>,
    pub structural: Option<bool>,
}

impl WallType {
    pub fn from_decoded(decoded: &DecodedElement) -> Self {
        let mut out = Self::default();
        for (field_name, value) in &decoded.fields {
            match (normalise_field_name(field_name).as_str(), value) {
                ("name", InstanceField::String(s)) => out.name = Some(s.clone()),
                ("kind" | "walltype" | "walltypekind", InstanceField::Integer { value, .. }) => {
                    out.kind = Some(WallKind::from_code(*value as u32));
                }
                ("function" | "wallfunction", InstanceField::Integer { value, .. }) => {
                    out.function = Some(WallFunction::from_code(*value as u32));
                }
                ("width" | "thickness", InstanceField::Float { value, .. }) => {
                    out.width_feet = Some(*value);
                }
                ("structural" | "isstructural", InstanceField::Bool(b)) => {
                    out.structural = Some(*b);
                }
                _ => {}
            }
        }
        out
    }

    /// Total thickness in inches — convenience for US-customary
    /// readouts in UIs and reports.
    pub fn width_inches(&self) -> Option<f64> {
        self.width_feet.map(|ft| ft * 12.0)
    }
}
