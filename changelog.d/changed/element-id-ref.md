- **`FieldType::ElementIdRef` is a class reference and nothing more
  (#1101, the maintainer, 2026-10-30).** A schema field's by-value reference
  is the type word `0e 00 00 00` and a `u16` naming the referenced class,
  six bytes; where its bit 15 is set the class is defined inline after it.
  The variant's `sub` read the two bytes after those six, which belong to
  whatever follows: the next field's name length on about 1,900 fields of a
  2024 to 2026 schema, the pad before an inline definition on about 240,
  and a class's GUID-list count on about 600. `sub` is gone from
  `ElementIdRef` and from `rvt-schema --json`, `referenced_tag` no longer
  keeps bit 15, and every reference to the root `ElementId` class reads as
  `ElementId`, where that used to depend on the next field.
  `FieldType::encode` writes the six bytes. Code that names `sub` stops
  compiling: drop it. No export changes (Measure run 41102077364, every
  reference model, the R2025 models, Autodesk's samples and the family
  corpus); the walker's schema scan finds fewer low-confidence candidates,
  which the default export already left out, so the diagnostics count fewer
  suppressed ones (1,138 to 91 `HostObjAttr` on Autodesk's 2027 MEP sample,
  on 39 files in all).
