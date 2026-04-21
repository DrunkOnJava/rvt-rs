//! Sheet rendering (VW1-11) — emit a 2D plan view of an `IfcModel`
//! as SVG.
//!
//! First-pass implementation: for each `BuildingElement` with an
//! `Extrusion` + `location_feet`, draw a rectangle sized to the
//! element's width × depth at its XY location. Element `ifc_type`
//! drives the stroke colour (walls black, doors blue, columns red,
//! etc.) so a plan looks recognizable without full geometry.
//!
//! Output is a self-contained SVG document — no external
//! stylesheets, no JS. Drop it in a browser, embed in a report,
//! or convert to PDF via any SVG-to-PDF tool.

use super::IfcModel;
use super::entities::IfcEntity;
use std::fmt::Write;

/// Options controlling sheet SVG output (VW1-11).
#[derive(Debug, Clone)]
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
/// The model bounding box is computed from each element's
/// `location_feet` ± half its `extrusion.width/depth`, then fit
/// to `options.width_px × options.height_px` preserving aspect.
///
/// Elements without an `extrusion` or `location_feet` are skipped
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
         fill=\"none\" stroke=\"#CCCCCC\" stroke-width=\"1\"/>",
        options.margin_px, options.margin_px, plot_w, plot_h,
    )
    .unwrap();

    for fp in &footprints {
        let sx = offset_x + ((fp.x - min_x) as f32 - fp.w as f32 * 0.5) * scale;
        let sy = offset_y + ((max_y - fp.y) as f32 - fp.d as f32 * 0.5) * scale;
        let sw = (fp.w as f32 * scale).max(1.0);
        let sh = (fp.d as f32 * scale).max(1.0);
        let (stroke, fill) = colors_for_ifc_type(&fp.ifc_type);
        write!(
            &mut out,
            "<rect x=\"{:.1}\" y=\"{:.1}\" width=\"{:.1}\" height=\"{:.1}\" \
             fill=\"{}\" stroke=\"{}\" stroke-width=\"1\"/>",
            sx, sy, sw, sh, fill, stroke
        )
        .unwrap();
        if options.show_labels {
            write!(
                &mut out,
                "<text x=\"{:.1}\" y=\"{:.1}\" font-size=\"9\" \
                 fill=\"#333333\" font-family=\"sans-serif\">{}</text>",
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
    x: f64,
    y: f64,
    w: f64,
    d: f64,
    ifc_type: String,
    name: String,
}

fn collect_footprints(model: &IfcModel) -> Vec<Footprint> {
    let mut out = Vec::new();
    for ent in &model.entities {
        if let IfcEntity::BuildingElement {
            ifc_type,
            name,
            location_feet,
            extrusion,
            ..
        } = ent
        {
            let Some(loc) = location_feet else {
                continue;
            };
            let Some(ext) = extrusion.as_ref() else {
                continue;
            };
            out.push(Footprint {
                x: loc[0],
                y: loc[1],
                w: ext.width_feet,
                d: ext.depth_feet,
                ifc_type: ifc_type.clone(),
                name: name.clone(),
            });
        }
    }
    out
}

fn bbox_of_footprints(fps: &[Footprint]) -> (f64, f64, f64, f64) {
    if fps.is_empty() {
        return (0.0, 0.0, 100.0, 100.0);
    }
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for fp in fps {
        let hx = fp.w * 0.5;
        let hy = fp.d * 0.5;
        min_x = min_x.min(fp.x - hx);
        min_y = min_y.min(fp.y - hy);
        max_x = max_x.max(fp.x + hx);
        max_y = max_y.max(fp.y + hy);
    }
    (min_x, min_y, max_x, max_y)
}

/// Per-category colour mapping (VW1-11). Sensible defaults that
/// match common drafting conventions — walls black, doors blue,
/// windows cyan, columns red, slabs grey.
fn colors_for_ifc_type(ifc_type: &str) -> (&'static str, &'static str) {
    match ifc_type {
        "IFCWALL" | "IFCWALLSTANDARDCASE" => ("#000000", "#EEEEEE"),
        "IFCDOOR" => ("#2266CC", "#DDE7FF"),
        "IFCWINDOW" => ("#22AACC", "#DDF0FF"),
        "IFCCOLUMN" => ("#CC2244", "#FFDDE2"),
        "IFCBEAM" | "IFCMEMBER" => ("#AA4499", "#F0DDE8"),
        "IFCSLAB" | "IFCROOF" | "IFCCOVERING" => ("#888888", "#EEEEEE"),
        "IFCSTAIR" | "IFCRAILING" => ("#AA7722", "#F5E5CC"),
        "IFCFURNITURE" | "IFCFURNISHINGELEMENT" => ("#228855", "#DDEEDD"),
        _ => ("#444444", "#F4F4F4"),
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
        assert!(svg.contains("stroke=\"#2266CC\""));
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
        assert_eq!(stroke, "#444444");
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
