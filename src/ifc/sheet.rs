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

#[cfg(test)]
mod tests {
    use super::super::Storey;
    use super::super::entities::{Extrusion, IfcEntity};
    use super::*;

    fn mk_wall(name: &str, loc: [f64; 3], w: f64, d: f64) -> IfcEntity {
        IfcEntity::BuildingElement {
            ifc_type: "IFCWALL".into(),
            name: name.into(),
            type_guid: None,
            predefined_type: None,
            storey_index: None,
            material_index: None,
            property_set: None,
            location_feet: Some(loc),
            rotation_radians: None,
            extrusion: Some(Extrusion {
                width_feet: w,
                depth_feet: d,
                height_feet: 10.0,
                profile_override: None,
            }),
            host_element_index: None,
            material_layer_set_index: None,
            material_profile_set_index: None,
            solid_shape: None,
            representation_map_index: None,
        }
    }

    /// The numbers of a path's `M` / `L` commands, as SVG points.
    fn path_points(svg: &str) -> Vec<(f32, f32)> {
        let d = svg
            .split("<path d=\"")
            .nth(1)
            .and_then(|rest| rest.split('"').next())
            .expect("a path");
        let numbers: Vec<f32> = d
            .split(['M', 'L', 'Z', ' '])
            .filter(|t| !t.is_empty())
            .map(|t| t.parse().unwrap())
            .collect();
        numbers.chunks_exact(2).map(|p| (p[0], p[1])).collect()
    }

    #[test]
    fn a_rotated_wall_is_drawn_turned() {
        // A 20 x 1 ft wall turned a quarter turn, beside a reference square.
        let mut wall = mk_wall("W", [0.0, 0.0, 0.0], 20.0, 1.0);
        if let IfcEntity::BuildingElement {
            rotation_radians, ..
        } = &mut wall
        {
            *rotation_radians = Some(std::f64::consts::FRAC_PI_4);
        }
        let model = IfcModel {
            entities: vec![wall],
            ..Default::default()
        };
        let svg = render_plan_svg(
            &model,
            &SheetOptions {
                show_labels: false,
                ..Default::default()
            },
        );
        assert_eq!(svg.matches("<path").count(), 1);
        let points = path_points(&svg);
        assert_eq!(points.len(), 4);
        // Turned 45 degrees, the wall's long sides run diagonally: its
        // corners are not an axis-aligned rectangle's.
        let xs: std::collections::BTreeSet<i64> =
            points.iter().map(|p| (p.0 * 10.0).round() as i64).collect();
        assert_eq!(xs.len(), 4);
    }

    #[test]
    fn a_slab_with_a_hole_is_drawn_with_the_hole_open() {
        let mut slab = mk_wall("S", [0.0, 0.0, 0.0], 0.0, 0.0);
        if let IfcEntity::BuildingElement {
            ifc_type,
            extrusion,
            ..
        } = &mut slab
        {
            *ifc_type = "IFCSLAB".into();
            *extrusion = Some(Extrusion::arbitrary_with_voids(
                vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.0, 10.0)],
                vec![vec![(4.0, 4.0), (6.0, 4.0), (6.0, 6.0), (4.0, 6.0)]],
                1.0,
            ));
        }
        let model = IfcModel {
            entities: vec![slab],
            ..Default::default()
        };
        let svg = render_plan_svg(&model, &SheetOptions::default());
        assert!(svg.contains("fill-rule=\"evenodd\""));
        assert_eq!(path_points(&svg).len(), 8, "outer ring and hole");
    }

    #[test]
    fn slabs_are_drawn_beneath_walls() {
        let mut slab = mk_wall("Slab", [0.0, 0.0, 0.0], 30.0, 30.0);
        if let IfcEntity::BuildingElement { ifc_type, .. } = &mut slab {
            *ifc_type = "IFCSLAB".into();
        }
        let model = IfcModel {
            entities: vec![mk_wall("Wall", [0.0, 0.0, 0.0], 10.0, 1.0), slab],
            ..Default::default()
        };
        let svg = render_plan_svg(&model, &SheetOptions::default());
        assert!(svg.find(">Slab<").unwrap() < svg.find(">Wall<").unwrap());
    }

    #[test]
    fn empty_model_still_produces_well_formed_svg() {
        let svg = render_plan_svg(&IfcModel::default(), &SheetOptions::default());
        assert!(svg.starts_with("<svg"));
        assert!(svg.ends_with("</svg>"));
        assert!(svg.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    }

    #[test]
    fn svg_includes_viewbox_and_dimensions() {
        let svg = render_plan_svg(
            &IfcModel::default(),
            &SheetOptions {
                width_px: 800,
                height_px: 600,
                ..SheetOptions::default()
            },
        );
        assert!(svg.contains("width=\"800\""));
        assert!(svg.contains("height=\"600\""));
        assert!(svg.contains("viewBox=\"0 0 800 600\""));
    }

    #[test]
    fn single_wall_produces_one_rect_plus_border_plus_background() {
        let model = IfcModel {
            entities: vec![mk_wall("Wall-1", [0.0, 0.0, 0.0], 10.0, 0.5)],
            ..Default::default()
        };
        let svg = render_plan_svg(&model, &SheetOptions::default());
        // Expected rects: background + border + one element.
        let rect_count = svg.matches("<rect").count();
        assert_eq!(rect_count, 3);
    }

    #[test]
    fn walls_use_wall_colour() {
        let model = IfcModel {
            entities: vec![mk_wall("W", [0.0, 0.0, 0.0], 10.0, 0.5)],
            ..Default::default()
        };
        let svg = render_plan_svg(&model, &SheetOptions::default());
        assert!(svg.contains("stroke=\"#000000\""));
    }

    #[test]
    fn doors_use_door_colour() {
        let model = IfcModel {
            entities: vec![IfcEntity::BuildingElement {
                ifc_type: "IFCDOOR".into(),
                name: "D1".into(),
                type_guid: None,
                predefined_type: None,
                storey_index: None,
                material_index: None,
                property_set: None,
                location_feet: Some([0.0, 0.0, 0.0]),
                rotation_radians: None,
                extrusion: Some(Extrusion {
                    width_feet: 3.0,
                    depth_feet: 0.5,
                    height_feet: 7.0,
                    profile_override: None,
                }),
                host_element_index: None,
                material_layer_set_index: None,
                material_profile_set_index: None,
                solid_shape: None,
                representation_map_index: None,
            }],
            ..Default::default()
        };
        let svg = render_plan_svg(&model, &SheetOptions::default());
        assert!(svg.contains("stroke=\"#D97706\""));
    }

    #[test]
    fn show_labels_false_omits_text_elements() {
        let model = IfcModel {
            entities: vec![mk_wall("Wall-1", [0.0, 0.0, 0.0], 10.0, 0.5)],
            ..Default::default()
        };
        let with_labels = render_plan_svg(
            &model,
            &SheetOptions {
                show_labels: true,
                ..SheetOptions::default()
            },
        );
        let no_labels = render_plan_svg(
            &model,
            &SheetOptions {
                show_labels: false,
                ..SheetOptions::default()
            },
        );
        assert!(with_labels.contains("<text"));
        assert!(!no_labels.contains("<text"));
    }

    #[test]
    fn svg_escapes_element_names() {
        let mut wall = mk_wall("Weird<&>Wall", [0.0, 0.0, 0.0], 10.0, 0.5);
        if let IfcEntity::BuildingElement { name, .. } = &mut wall {
            *name = "Weird<&>Wall".into();
        }
        let model = IfcModel {
            entities: vec![wall],
            ..Default::default()
        };
        let svg = render_plan_svg(&model, &SheetOptions::default());
        assert!(svg.contains("Weird&lt;&amp;&gt;Wall"));
        assert!(!svg.contains("Weird<&>Wall"));
    }

    #[test]
    fn elements_without_extrusion_or_location_are_skipped() {
        let mut no_ext = mk_wall("W", [0.0, 0.0, 0.0], 10.0, 0.5);
        if let IfcEntity::BuildingElement { extrusion, .. } = &mut no_ext {
            *extrusion = None;
        }
        let mut no_loc = mk_wall("W2", [0.0, 0.0, 0.0], 10.0, 0.5);
        if let IfcEntity::BuildingElement { location_feet, .. } = &mut no_loc {
            *location_feet = None;
        }
        let model = IfcModel {
            entities: vec![no_ext, no_loc],
            ..Default::default()
        };
        let svg = render_plan_svg(&model, &SheetOptions::default());
        // background + border only, no element rects.
        assert_eq!(svg.matches("<rect").count(), 2);
    }

    #[test]
    fn bbox_handles_empty_footprints_without_nan() {
        let (mn_x, mn_y, mx_x, mx_y) = bbox_of_footprints(&[]);
        assert!(mn_x.is_finite() && mn_y.is_finite());
        assert!(mx_x.is_finite() && mx_y.is_finite());
    }

    #[test]
    fn sheet_options_default_has_white_background() {
        let opts = SheetOptions::default();
        assert_eq!(opts.background.as_deref(), Some("#FFFFFF"));
        assert!(opts.show_labels);
    }

    #[test]
    fn transparent_background_omits_fill_rect() {
        let svg = render_plan_svg(
            &IfcModel::default(),
            &SheetOptions {
                background: None,
                ..SheetOptions::default()
            },
        );
        // Only the border rect remains (no background).
        assert_eq!(svg.matches("<rect").count(), 1);
    }

    #[test]
    fn colors_unknown_type_falls_back_to_default_grey() {
        let (stroke, _) = colors_for_ifc_type("IFCMYSTERYELEMENT");
        assert_eq!(stroke, "#4C4E52");
    }

    #[test]
    fn multi_element_model_produces_expected_rect_count() {
        let model = IfcModel {
            entities: vec![
                mk_wall("W1", [0.0, 0.0, 0.0], 10.0, 0.5),
                mk_wall("W2", [20.0, 0.0, 0.0], 10.0, 0.5),
                mk_wall("W3", [0.0, 15.0, 0.0], 0.5, 30.0),
            ],
            ..Default::default()
        };
        let svg = render_plan_svg(&model, &SheetOptions::default());
        // background + border + 3 elements = 5
        assert_eq!(svg.matches("<rect").count(), 5);
    }

    #[test]
    fn compile_with_storey_import() {
        // Makes sure the `Storey` import from super::super compiles;
        // viewers that pass in a building_storeys list should work
        // without modification (storeys are metadata, not drawn).
        let model = IfcModel {
            building_storeys: vec![Storey {
                name: "Ground".into(),
                elevation_feet: 0.0,
            }],
            ..Default::default()
        };
        let _svg = render_plan_svg(&model, &SheetOptions::default());
    }
}
