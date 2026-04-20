//! Per-class element decoders.
//!
//! Each submodule implements [`crate::walker::ElementDecoder`] for
//! one Revit class. Adding a new class is a three-file change:
//!
//! 1. Add `mod my_class;` here.
//! 2. Register it in [`all_decoders`] so the walker dispatch table
//!    picks it up.
//! 3. Implement `ElementDecoder` in `src/elements/my_class.rs` —
//!    see `level.rs` as the reference example.
//!
//! See `docs/extending-layer-5b.md` for the contributor walkthrough.
//!
//! # Relationship to the generic decoder
//!
//! [`crate::walker::decode_instance`] is the generic fallback — it
//! walks any class's declared fields using the schema's
//! `FieldType` classifications. It always works, but the output is
//! untyped (each field shows up as an `InstanceField` variant).
//!
//! Concrete decoders in this module add a typed layer on top: they
//! call `decode_instance` first, then pattern-match the `fields`
//! vector into a class-specific struct (e.g. `Level { name,
//! elevation, is_building_story, … }`). Callers who want typed
//! Wall / Floor / Door values use these; callers who want a
//! uniform untyped dump use `decode_instance` directly.

pub mod category;
pub mod ceiling;
pub mod floor;
pub mod grid;
pub mod level;
pub mod openings;
pub mod reference_planes;
pub mod reference_points;
pub mod roof;
pub mod styling;
pub mod wall;

use crate::walker::ElementDecoder;

/// Every registered [`ElementDecoder`] in insertion order.
///
/// The walker dispatch table is built from this list at runtime.
/// Future registration helpers (inventory crate, etc.) could replace
/// this with compile-time discovery; for now an explicit Vec keeps
/// it obvious what ships with the crate.
pub fn all_decoders() -> Vec<Box<dyn ElementDecoder>> {
    vec![
        Box::new(level::LevelDecoder),
        Box::new(category::CategoryDecoder),
        Box::new(category::SubcategoryDecoder),
        Box::new(styling::MaterialDecoder),
        Box::new(styling::FillPatternDecoder),
        Box::new(styling::LinePatternDecoder),
        Box::new(styling::LineStyleDecoder),
        Box::new(reference_points::BasePointDecoder),
        Box::new(reference_points::SurveyPointDecoder),
        Box::new(reference_points::ProjectPositionDecoder),
        Box::new(grid::GridDecoder),
        Box::new(grid::GridTypeDecoder),
        Box::new(reference_planes::ReferencePlaneDecoder),
        Box::new(wall::WallDecoder),
        Box::new(wall::WallTypeDecoder),
        Box::new(floor::FloorDecoder),
        Box::new(floor::FloorTypeDecoder),
        Box::new(roof::RoofDecoder),
        Box::new(roof::RoofTypeDecoder),
        Box::new(ceiling::CeilingDecoder),
        Box::new(ceiling::CeilingTypeDecoder),
        Box::new(openings::DoorDecoder),
        Box::new(openings::WindowDecoder),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_decoders_includes_level() {
        let decoders = all_decoders();
        let names: Vec<&str> = decoders.iter().map(|d| d.class_name()).collect();
        assert!(names.contains(&"Level"));
    }

    #[test]
    fn all_decoders_includes_category_and_subcategory() {
        let decoders = all_decoders();
        let names: Vec<&str> = decoders.iter().map(|d| d.class_name()).collect();
        assert!(names.contains(&"Category"));
        assert!(names.contains(&"Subcategory"));
    }

    #[test]
    fn decoder_class_names_are_unique() {
        let decoders = all_decoders();
        let mut seen = std::collections::BTreeSet::new();
        for d in &decoders {
            assert!(
                seen.insert(d.class_name()),
                "duplicate decoder for class {}",
                d.class_name()
            );
        }
    }
}
