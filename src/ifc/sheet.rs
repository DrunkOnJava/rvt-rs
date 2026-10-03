//! Sheet rendering (VW1-11) — emit a 2D plan view of an `IfcModel`
//! as SVG.
//!
//! Each placed `BuildingElement` with a body is drawn as what that body
//! covers in plan ([`super::body_geometry::plan_outline`]): an extrusion's
//! profile turned by the element's rotation, holes left open, and the
//! convex hull of a swept, revolved or brep solid. An axis-aligned
//! rectangle stays a `<rect>`; every other outline is a `<path>`. Element
//! `ifc_type` drives the stroke colour (walls black, doors amber, columns
//! red, etc.).
//!
//! Output is a self-contained SVG document — no external
//! stylesheets, no JS. Drop it in a browser, embed in a report,
//! or convert to PDF via any SVG-to-PDF tool.

use super::IfcModel;
use super::body_geometry::{Body, Placement, Ring, element_body, plan_outline};
use super::entities::IfcEntity;
use std::fmt::Write;

/// Options controlling sheet SVG output (VW1-11).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SheetOptions {
    /// Plot width in SVG user units (pixels by default).
    pub width_px: u32,
    /// Plot height in SVG user units.
    pub height_px: u32,
    /// Margin inside the SVG viewBox, in user units.
    pub margin_px: f32,
    /// Show element name labels? (Large plans often want them off.)
    pub show_labels: bool,
    /// Background fill colour. `None` = transparent.
    pub background: Option<String>,
}

impl Default for SheetOptions {
    fn default() -> Self {
        Self {
            width_px: 1200,
            height_px: 800,
            margin_px: 40.0,
            show_labels: true,
            background: Some("#FFFFFF".into()),
        }
    }
}

/// Render `model` as an SVG plan view (VW1-11). Returns a string
/// ready to write to a `.svg` file or embed inline.
///
/// The plan is a top-down projection: X maps to SVG x, Y maps to
/// SVG y (flipped so +Y runs up, matching drafting conventions).
/// The model bounding box is the box of every element's plan outline,
/// fit to `options.width_px × options.height_px` preserving aspect.
///
/// Elements without a body or a `location_feet` are skipped
/// (nothing to draw).
pub fn render_plan_svg(model: &IfcModel, options: &SheetOptions) -> String {
    let footprints = collect_footprints(model);
    let (min_x, min_y, max_x, max_y) = bbox_of_footprints(&footprints);
    let world_w = (max_x - min_x).max(1e-6) as f32;
    let world_h = (max_y - min_y).max(1e-6) as f32;

    let plot_w = options.width_px as f32 - 2.0 * options.margin_px;
    let plot_h = options.height_px as f32 - 2.0 * options.margin_px;
    let scale = (plot_w / world_w).min(plot_h / world_h);
    let offset_x = options.margin_px + (plot_w - world_w * scale) * 0.5;
    let offset_y = options.margin_px + (plot_h - world_h * scale) * 0.5;

    let mut out = String::with_capacity(1024 + footprints.len() * 128);
    write!(
        &mut out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" \
         height=\"{}\" viewBox=\"0 0 {} {}\">",
        options.width_px, options.height_px, options.width_px, options.height_px
    )
    .unwrap();
    if let Some(bg) = options.background.as_deref() {
        write!(
            &mut out,
            "<rect width=\"{}\" height=\"{}\" fill=\"{}\"/>",
            options.width_px, options.height_px, bg
        )
        .unwrap();
    }
    // Border around the plot area so the sheet feels like a drawing.
    write!(
        &mut out,
        "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
         fill=\"none\" stroke=\"#CDCED0\" stroke-width=\"1\"/>",
        options.margin_px, options.margin_px, plot_w, plot_h,
    )
    .unwrap();

    let to_svg = |(x, y): (f64, f64)| {
        (
            offset_x + (x - min_x) as f32 * scale,
            offset_y + (max_y - y) as f32 * scale,
        )
    };
    for fp in &footprints {
        let (x0, y0, x1, y1) = ring_bounds(&fp.outer);
        let (sx, sy) = to_svg((x0, y1));
        let sw = ((x1 - x0) as f32 * scale).max(1.0);
        let sh = ((y1 - y0) as f32 * scale).max(1.0);
        let (stroke, fill) = colors_for_ifc_type(&fp.ifc_type);
        if fp.axis_aligned_rectangle {
            write!(
                &mut out,
                "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
                 fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
                sx, sy, sw, sh, fill, stroke
            )
            .unwrap();
        } else {
            let mut d = String::new();
            for ring in std::iter::once(&fp.outer).chain(&fp.holes) {
                for (i, &p) in ring.iter().enumerate() {
                    let (px, py) = to_svg(p);
                    write!(
                        &mut d,
                        "{}{:.1} {:.1} ",
                        if i == 0 { "M" } else { "L" },
                        px,
                        py
                    )
                    .unwrap();
                }
                d.push_str("Z ");
            }
            write!(
                &mut out,
                "<path d=\"{}\" fill-rule=\"evenodd\" fill=\"{}\" stroke=\"{}\" \
                 stroke-width=\"1\"/>",
                d.trim_end(),
                fill,
                stroke
            )
            .unwrap();
        }
        if options.show_labels {
            write!(
                &mut out,
                "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"9\" \
                 fill=\"#2C2E30\" font-family=\"sans-serif\">{}</text>",
                sx + sw * 0.5,
                sy + sh * 0.5,
                xml_escape(&fp.name)
            )
            .unwrap();
        }
    }
    out.push_str("</svg>");
    out
}

struct Footprint {
    /// Plan outline in model feet, and its holes.
    outer: Ring,
    holes: Vec<Ring>,
    /// A rectangle on the model axes, drawn as a `<rect>`.
    axis_aligned_rectangle: bool,
    ifc_type: String,
    name: String,
}

fn collect_footprints(model: &IfcModel) -> Vec<Footprint> {
    let mut out = Vec::new();
    for ent in &model.entities {
        let IfcEntity::BuildingElement {
            ifc_type,
            name,
            location_feet,
            rotation_radians,
            ..
        } = ent
        else {
            continue;
        };
        // An opening is a void in its host, not something drawn (RE-84).
        if ifc_type == "IFCOPENINGELEMENT" {
            continue;
        }
        let Some(location) = location_feet else {
            continue;
        };
        let Some(body) = element_body(ent, &model.representation_maps) else {
            continue;
        };
        let placement = Placement::new(Some(*location), *rotation_radians);
        let Some((outer, holes)) = plan_outline(body, &placement) else {
            continue;
        };
        let on_axes = placement.sin.abs() < 1e-12 || placement.cos.abs() < 1e-12;
        let axis_aligned_rectangle = on_axes
            && matches!(body, Body::Extrusion(extrusion) if extrusion.profile_override.is_none());
        out.push(Footprint {
            outer,
            holes,
            axis_aligned_rectangle,
            ifc_type: ifc_type.clone(),
            name: name.clone(),
        });
    }
    // Plates and spaces underlie the plan: drawn after the walls, their
    // fills would hide every wall, column and door on their storey.
    out.sort_by_key(|fp| !is_underlay(&fp.ifc_type));
    out
}

/// Types drawn beneath the rest of the plan.
fn is_underlay(ifc_type: &str) -> bool {
    matches!(
        ifc_type,
        "IFCSLAB" | "IFCROOF" | "IFCCOVERING" | "IFCSPACE" | "IFCPLATE"
    )
}

fn ring_bounds(ring: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    ring.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |(x0, y0, x1, y1), &(x, y)| (x0.min(x), y0.min(y), x1.max(x), y1.max(y)),
    )
}

fn bbox_of_footprints(fps: &[Footprint]) -> (f64, f64, f64, f64) {
    if fps.is_empty() {
        return (0.0, 0.0, 100.0, 100.0);
    }
    fps.iter().map(|fp| ring_bounds(&fp.outer)).fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |(a, b, c, d), (x0, y0, x1, y1)| (a.min(x0), b.min(y0), c.max(x1), d.max(y1)),
    )
}

/// Per-category colour mapping (VW1-11), from the viewer design system's
/// ramps: walls black, windows blue, doors amber, columns and beams red,
/// slabs gray, stairs dark amber and furniture green. Each fill is the
/// light tint of its stroke's ramp.
fn colors_for_ifc_type(ifc_type: &str) -> (&'static str, &'static str) {
    match ifc_type {
        "IFCWALL" | "IFCWALLSTANDARDCASE" => ("#000000", "#E4E5E7"),
        "IFCDOOR" => ("#D97706", "#FFFBEB"),
        "IFCWINDOW" => ("#1881FF", "#E6F1FF"),
        "IFCCOLUMN" => ("#DC2626", "#FEE2E2"),
        "IFCBEAM" | "IFCMEMBER" => ("#F87171", "#FEF2F2"),
        "IFCSLAB" | "IFCROOF" | "IFCCOVERING" => ("#909398", "#F5F5F7"),
        "IFCSTAIR" | "IFCRAILING" => ("#92400E", "#FEF3C7"),
        "IFCFURNITURE" | "IFCFURNISHINGELEMENT" => ("#16A34A", "#F0FDF4"),
        _ => ("#4C4E52", "#FAFAFA"),
    }
}

fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            other => out.push(other),
        }
    }
    out
}
