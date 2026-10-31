- **Public API nothing in the workspace used is removed, as planned before
  1.0 (the maintainer, 2026-10-30).** No exporter, CLI, binding, test,
  example, benchmark or fuzz target called these, so 1.0 doesn't promise
  them. Most were convenience wrappers over fields that stay public:
  - `Floor::is_depressed`, `Door::is_flipped`, `Window::sill_height_inches`,
    `Wall::is_unconnected` and `WallType::width_inches`: read the field
    they wrapped.
  - `WallKind::is_basic` and `WallFunction::to_ifc_predefined`: match on the
    enum.
  - `FloorBoundaryLoop::area_sqft` and `to_curve_loop`: the loop's points
    stay public, so compute the area from them.
  - `Extrusion::t_shape`, `l_shape`, `u_shape`, `rectangle_hollow`,
    `circle_hollow` and `arbitrary_with_voids`: build the `ProfileDef`
    variant directly.
  - `MaterialLayerSet::total_thickness_feet`: sum the layers.
  - `ifc::category_map::is_ifc_member`, `EvidenceTier::meets_verified_gate`,
    `InflateShortfall::is_complete` and `RoundTripReport::is_typed_clean`.
  - `RevitFile::preview_png_untrimmed`, `preview_raw` and
    `transmission_data_probe`: `preview_png` remains, and
    `transmission_data::probe_transmission_data` is what the last one
    wrapped.
  - `WalkerControl::cancel_token`: keep a clone of the `CancelToken` you
    pass to `with_cancel`.
  - `SchemaTable::tagged_ancestor_map`, `walker::encode_adocument_fields`
    and `ADocumentInstance::elem_table_pointer`.
