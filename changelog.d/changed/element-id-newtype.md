- **Breaking: element ids are an `rvt::ElementId` (#873).** The ids the
  element structs carry (`level_id`, `host_id`, `type_id`, `symbol_id` and
  the rest, 95 fields), `OpeningHostRelationship::host_element_id` and the
  arc-wall record's ids are `Option<rvt::ElementId>` where they were
  `Option<u32>`. `ElementId` wraps the `i64` the Revit API's
  `ElementId.Value` is, and serialises as the bare number, so the JSON the
  CLIs print and the Python values are the same integers. `ElementId::from`
  takes a `u32`, `from_raw` a file's 64-bit slot (0 and values from
  `u32::MAX` up name no element), `get` gives the value and `as_u32` the
  stored form. The wire-level views keep the `u32` the file stores:
  `walker::DecodedElement::id`, `walker::InstanceField::ElementId` and
  `elem_table::ElemRecord`; `DecodedElement::element_id()` gives the typed
  id.
