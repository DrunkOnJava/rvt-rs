# RE-260: Revit 2008 to 2010 element records, the material names of Revit 2008 to 2011, and the Levels of Revit 2008 to 2012

**Date:** 2026-10-28
**Issues:** #421 (element records of every release); follows RE-259 (Revit 2008 to 2013 files open, and 2011 to 2014 element records are read in the 32-bit layout of Revit 2015 to 2023).
**Artefacts:** local files written by Revit 2008 (one, build 20070324_1700, the release taken from the build date), 2009 (two, builds 20080126_1900 and 20080321_1900, likewise), 2010 (one, build 20090317_2115), 2011 (four) and 2012 (two), RE-259's. **They are closed source and are measured locally only. None may be published, attached or fetched by CI.** Autodesk's download host has no sample project before 2015.
**Probe:** `examples/probe_re260_revit_2008_2010.rs`.
**Status:** positive on each file's own consistency. **There is no Revit export, and no copy of the same model saved by another release, of any 2008 to 2010 file here.** What is shown is that the 2011 layout reads them without a byte left over, and that what it reads hangs together.

## 1. Before

With RE-259 the files of Revit 2008 to 2010 open and their schemas read, but their element records are not read: `rvt-ifc` writes the spatial scaffold and the names the schema walker takes for materials. The 2011 files' records are read, but none of their materials is named: the name rules of 2012 and later find nothing on them. No 2008 to 2012 file has a storey: their Levels' records are read, but not their names and elevations.

## 2. The envelope

| file | release | markers | id before the marker | none | bytes between category and placement kind |
|---|---|---:|---|---:|---|
| local project | 2008 | 120 | 48 bytes: 118 | 2 | 12, all `ff`, on all 118 |
| local project | 2009 | 171 | 52 bytes: 170 | 1 | 16, all `ff`, on all 170 |
| local stub | 2009 | 0 | | | |
| local schedule | 2010 | 12 | 52 bytes: 12 | 0 | 16, all `ff`, on all 12 |
| local files (4) | 2011 | 39 to 106 | 52 bytes: all | 0 | 16, all `ff`, on all |

2009 and 2010 frame their records as 2011 to 2023 do. A 2008 record is the 2009 record with 4 bytes fewer between the category and the placement kind: id, size, `ElementHeader` tag and count, then the category 14 bytes after the id, as on 2009; from the placement kind to the marker every byte is where 2009's is. Every one of those bytes is `ff` on both releases, so which four 2008 leaves out is not shown, and the container reference, read 22 bytes before the marker as from 2009 on, is unset on every 2008 record. `envelope_shortfall` gives 4 on 2008.

`REVIT_32BIT_RELEASES` is now 2008 to 2023, and `schema_tags` gains rows for 2008, 2009 and 2010, read from each file's schema:

| class | 2008 | 2009 | 2010 |
|---|---|---|---|
| `CellList` | 0x0179 | 0x01a3 | 0x01b1 |
| `Material` | 0x04fb | 0x0556 | 0x05b5 |
| `PhysicalParamSet` | 0x0615 | 0x06a8 | 0x06e4 |
| `PatternHelper` | 0x00fd | 0x011d | 0x012c |
| `VerticalRegionsStructure` | 0x0864 | 0x094d | 0x09e0 |

The two 2009 files have the same schema. RE-259's two 2011 to 2013 layouts hold on 2008 and 2009: every shading frame of the local 2008 and 2009 files puts 24 bytes between the 0.5 and the colour (58 and 79 frames), and an ElementId and the value-block mark open `00 00 00 00 00 01` 4 and 20 times. The 2010 file has neither.

## 3. Material names before 2012

From Revit 2012 a material's name ends before `PhysicalParamSet`'s tag (RE-116, RE-180). On 2008 to 2011 nothing ends it there. The name is `u32 n · UTF-16 × n` followed directly by the material's properties, which open with one `f64` written three times: a material of a 2011 file is followed by `6f 34 f3 6f 04 c2 ed 40` three times. Before the name come zero or more parameter entries (a BuiltInParameter id and a string, such as -1010106 `Rendering appearance not upgraded`). Some materials also frame the name as a `MaterialElemGroupHelper` (`ff ff ff ff`, that class's tag, the string).

`name_before_properties` takes the first name in the material object that three equal `f64`s follow, on releases before 2012:

| file | release | material objects | named before | named now |
|---|---|---:|---:|---:|
| local project | 2008 | 101 | not read | 101 |
| local project | 2009 | 125 | not read | 125 |
| local stub | 2009 | 9 | not read | 9 |
| local schedule | 2010 | 1 | not read | 1 |
| local files (4) | 2011 | 68 / 47 / 44 / 137 | 0 | all |

Where the object's own `MaterialElemGroupHelper` frame also names it, the two names agree: 15 of 15 and 27 of 27 on two 2011 files, 1 of 1 on 2010. On 2008 that frame comes after the name and holds an empty string. On the 2012 and 2013 files the rule finds no name, and it is not applied there.

The names are those of Revit's own templates (`Default`, `Default Wall`, `Default Roof`, `Glass`, `Air Barrier - Air Infiltration Barrier`) and the manufacturers' finishes the files' content brings. With the shading frame of §2, every material of the 2008 and 2009 projects has a colour (101 of 101, 125 of 125).

## 4. Levels before 2013

A Level's name block (RE-51, RE-107) is its ElementId, a run of `ff`, a pad and the name, and its elevation follows the reference to its `Plane`. From 2013 that reference is `05 00 00 00` and the `Plane` tag, unframed, and `schema_elevation_marker` finds it. On 2008 to 2012 it is `ff ff ff ff` and the `Plane` tag, 6 bytes after the name on every Level measured, so no 2008 to 2012 Level had an elevation, and none became a storey.

After that frame comes one reference slot, `00 00 00 00` when unset or `ff ff ff ff` and a class tag when set, then the elevation 16 bytes on, and the same `f64` again 24 bytes after it. `framed_plane_marker` and `FRAMED_PLANE_UNTIL` read it on releases before 2013:

| file | release | Levels |
|---|---|---|
| local project | 2008 | Level 1 0 ft, Level 2 10 ft |
| local projects (2) | 2009 | the same |
| local schedule | 2010 | the same |
| local files (4) | 2011 | the same |
| local project | 2012 | the same |
| local project | 2012 | Level 0 0 ft, Level 1 13.123 ft (4 m, the one set slot) |

Level 1 at 0 ft and Level 2 at 10 ft are the Levels of Revit's imperial template.

RE-118's Building Story setting, measured from 2023 on, is not read before 2013. Where its anchor is found on these files, the two 2012 projects, it reads "not a building story" on all four Levels, as no template Level is, and the export would then have had no storey; on 2008 to 2011 the anchor is not found. Every 2008 to 2012 Level is a storey. The 2009 showcase's two rows of doors stand at 0 and 10 ft. 2013 and 2014 read their Levels as before (the 2013 sample's five, the 2014 project's four).

## 5. What the records give

- **2008** (a reinforced-concrete box causeway): one placed instance, a generic model 101.9 by 33.5 by 23.4 ft, held in four partitions under one ElementId; every other record is a type.
- **2009** (a door showcase): 8 doors and 1 wall, each held in up to three partitions under one ElementId. The wall is 19 ft long, 8 in thick and 20 ft tall. The doors are 3 by 7 ft, all on the wall's line, in two rows on Levels 1 and 2 (0 and 10 ft), all within the wall's length, and the export voids and fills the wall for each of them.
- **2010** (a schedule): two placed instances, of categories -2001271 and -2001272, which no IFC class is mapped to; the rest are types. It exports its one material. The schema walker named 18 materials there before; they are not materials but fill patterns (`Aluminum`, `Concrete` and `Wood 1` follow `FillPattern`'s tag) and a category (`Glass`).
- **2011**: as RE-259 found, now with their materials named: 6 furniture on two files, 6 furniture and 4 lighting fixtures on one, 3 electrical equipment on one.

`rvt-ifc` exports the 2008 causeway as one `IfcBuildingElementProxy` and 101 materials, and the 2009 showcase as 8 `IfcDoor` and 1 `IfcWall` with 125 materials, each door voiding and filling the wall.

The 2012 projects now export their storeys: one 2012 project's 18 elements on its Level 0 and Level 1 (0 and 4 m), where before they were on the fallback storey. The 2009 showcase puts its wall and 4 doors on Level 1 and its other 4 doors on Level 2. The 2008, 2009 and 2012 exports pass IfcOpenShell 0.9.0's validation with no issue.

**Nothing else moves.** `rvt-ifc` was run with this branch and with RE-259's on the same 132 distinct local files as RE-259 §8. With the timestamped lines set aside, the 122 exports of 2013 to 2027 are byte-identical. The 10 that change are the files of 2008 to 2012.

## 6. What is not shown

- **Any oracle for 2008 to 2010.** No Revit export and no copy of the same model saved by another release is held. The records read as 2011's do, the elements they give hang together, and the exports validate; whether each field is Revit's is not measured.
- **Level elevations against Revit.** The values read are Revit's template Levels on all but one file, and 4 m on that one; no Revit output confirms them.
- **Which four bytes a 2008 envelope leaves out.** They are all `ff` on 2009 as on 2008. A 2008 element inside a group or another container would show it.
- **The releases of the 2008 and 2009 files.** They are taken from the build date (RE-259 §2).
- **Any public evidence.** Every file here is closed source; the measurements cannot be repeated on CI.
- Anything 2015 to 2023 do not read: design options, layered joins, IFC export overrides, parameters.

## 7. Reproduce

```bash
cargo run --release --example probe_re260_revit_2008_2010 -- file.rvt [more.rvt ...]
```
