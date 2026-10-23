# RE-167: a type object's GlobalId comes from the instance's original symbol

**Backlog:** B60, B72, B73. **Status:** positive. **Shipped in:** #663 (family instance types) and #666 (door and window types), both 2026-10-04.

**Written:** 2026-10-23, from the record of #663 and #666, because the code cited RE-167 without a report. Nothing was re-measured for this report.

**Evidence:** red runs on draft #662 (run 37219969064) and draft #665, with the output in each PR's test commit; green Corpus health runs 37224351714 and 37225506986. The real-file targets are `tests/type_global_ids.rs` (`type_objects_take_revits_global_ids`) and `tests/door_type_global_ids.rs` (`door_and_window_types_take_revits_global_ids`). No probe: the targets are the check.

**Files:** the four RE1 models (`ifc-ecs-re1-*.rvt`) against Revit's IFC2X3 exports of them, and Core Interior (`magnetar-2024-core-interior.rvt`, sha256 `c805df44…`) against Revit's `2024_Core_Interior_slim.ifc` (`bfdf36ff…`). Full hashes are in `research/witness-registry.json`.

## 1. The original symbol

Revit gives a family instance's type object the GlobalId of the instance's original symbol (`ExporterIFCUtils.GetOriginalSymbol` in Revit's API), and writes one type object per original symbol, keeping the symbol's Tag. The instance's `GElement` data object holds that symbol's ElementId 20 bytes before its end: a `FamilySymbol`, `SysMullionFamSym` or `SysPanelFamSym`. Where an instance's copies disagree, the copy in the highest-numbered partition decides (Core Interior's doors).

An instance that uses its own geometry (`0xffffffff` in that slot) takes a sub-element GlobalId of itself, as revit-ifc's `InstanceAsType` does. A type Revit writes under several entities keeps its own GlobalId on the group holding its lowest ElementId, and the others are hashed from the entity and predefined type.

## 2. Doors and windows

Revit hashes a door's or window's type GlobalId from its original symbol's GlobalId and the door's flip: the MD5 of `<symbol GlobalId>Sub-element:Flipped: <True|False>`, followed by ` InAssembly: False` from revit-ifc 25.4 on (a Revit 2025 or later file; Core Interior's 2024 export has no suffix). The flip comes from the door's box centre and facing against its host wall's axis, as revit-ifc's `DoorWindowInfo` does; a door with no host is not flipped. A door type is written once per flip.

## 3. Measured

Both targets pass on RE1 Architecture, Electrical, Mechanical and Plumbing and on Core Interior: every element typed in both rvt-rs's export and Revit's has its type object's GlobalId equal to Revit's. The targets compare only elements typed in both, so they do not show that rvt-rs types every element Revit types. Core Interior's witness observation changed with them, from IfcColumnType 1 to 175, IfcSlabType 5 to 7 and IfcDoorType 3 to 10, and IfcOpenShell validates the export with no issues.

## 4. Not established

- At #666 the typed-no-geometry export mode wrote no type objects, so no flip-dependent GlobalId was written there (B74). #694 changed that: the mode keeps each element's type object, entity, GlobalId and name, as `tests/typed_no_geometry_types.rs` checks.
