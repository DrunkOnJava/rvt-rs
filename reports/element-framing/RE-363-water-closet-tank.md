# RE-363: What separates the water closet tank Revit's export lacks from the fixtures it holds

**Date:** 2026-11-08
**Issues:** #1496 (RE1 Plumbing's water closet tank 442378 is written, and Revit's export holds neither it nor its type).
**Artefacts:** `RE1-Plumbing.rvt`, sha256 `03689df2aa426031a216e65051b9f19d3265d6fe96884dd47373b17435f9e54c`, against `RE1-Plumbing.ifc`, sha256 `294490b222632509779910345f27a4eeda3645956138a593298c241495f7b8b6` (Revit 26.2, IFC2X3); every other model Measure has with a Revit export (51 models with the plumbing probe, 52 with the empty-body and type-level probes; the held-out models as counts only).
**Scorers:** `tools/re/tank_rules_vs_ifc.py`, `tools/re/empty_bodies_vs_ifc.py`.
**Runs:** Measure 42518898114 (head af85afad) and 42521763730 (head c5bd28e3).
**Status:** positive. A family instance whose saved graphics decode cleanly to no surface, with a branch left out as not visible, is not in Revit's export on any model measured. The rule is written for plumbing fixtures only (section 7).

## 1. The question

rvt-rs writes a family instance whenever its record box has a volume (RE-21). Revit's exporter writes a family instance that is not a door or a window only when its 3D body has a solid or a mesh (`FamilyInstanceExporter` in revit-ifc, CLEANROOM.md source 5). The tank is the one plumbing fixture of RE1 Plumbing Revit's export lacks.

## 2. Rules on plumbing fixtures

`tools/re/tank_rules_vs_ifc.py` over every model with an export. A rule lands only with no false negative, which is a fixture in Revit's export that the rule would drop.

| rule | drops | false negatives | of Revit's lacked fixtures |
|---|---:|---:|---:|
| `bodiless_by_forms_or_graphics` | 1 | 0 | 1 |
| `graphics_no_surface_and_no_diagnostic` | 1 | 0 | 1 |
| `graphics_all_hidden` | 39 | 38 | 1 |
| `graphics_no_surface_at_all` | 49 | 48 | 1 |
| `forms_family_nests_instance_or_unread` | 72 | 71 | 1 |
| `host_unset` | 193 | 192 | 1 |
| `work_plane_unset` | 363 | 362 | 1 |
| `nested_mutual_any_category` | 194 | 194 | 0 |

The record fields (host, work plane, mutual references) and the family's forms do not separate them: the rules that read them and drop the tank also drop 71 to 362 fixtures Revit writes, and the mutual-reference rule drops 194 Revit writes and not the tank. The container slot and design option rules drop nothing. Of the graphics rules only the two that need no surface and no diagnostic have no false negative; a broader reading of the graphics drops 38 or 48 fixtures Revit writes. The tank's graphics at Fine detail decode with no surface, no diagnostic, and every primitive under a branch flagged not visible.

## 3. The same test on every family instance

`tools/re/empty_bodies_vs_ifc.py`, 52 models, 31,958 family instances other than doors and windows, of which 6,526 are not in Revit's export:

| rule | drops | false negatives | dropped, not in the export |
|---|---:|---:|---:|
| saved graphics decode to no surface, no diagnostic | 5,553 | 2 | 5,551 |
| the same, or only view-component diagnostics | 5,553 | 2 | 5,551 |
| the same, and a branch left out as not visible | 5,551 | **0** | 5,551 |
| read once per type symbol, the same test | 5,556 | **0** | 5,556 |

The two false negatives of the first two rules are one element on each terrain variant of the BIM4LCA office, and the requirement that a branch was left out as not visible is what removes them. An instance with no saved graphics at all is kept.

What the type-level rule drops, by model: the BIM4LCA office's 2,685 curtain panels on each of its two files, 89 parking spaces on each terrain variant, RE1 Plumbing's tank, and on the held-out models 1 on the flowbim house and 6 on Frigento (counts only). A door or window is counted apart: revit-ifc writes one with no geometry, so no rule here touches it.

## 4. Cost

Reading every instance's saved graphics costs 31% to 36% of an office export. An instance draws its type symbol's graphics, so `probe_re363_type_level` reads only the symbols: 534 ms against a 13,082 ms export on the BIM4LCA office as built (55 symbols requested), about 4%.

## 5. What is not established

- The rule in any other category (section 7).
- The type-level rule drops 5 more instances than the instance-level one, all 5 not in Revit's export. A type whose instances differ is seen only by the instance-level reading, so a rule read per type could differ from the instance's own body on a model not measured here.
- The office's column 1927742, the other extra of #1496, is not dropped by this rule on the permit file: its 2,685 drops are all curtain panels.

## 7. The rule as written

A plumbing fixture whose type's saved graphics decode cleanly to no surface, with a branch left out as not visible, is left out and counted as `element_record_family_type_draws_nothing` (`types_without_visible_body`, `without_invisible_plumbing_fixture_types`). It is the type-level reading of section 3, on one category.

**Why one category.** A first version applied it to every family instance but doors and windows. Measure run 42532619862 (head `8979e20c`, base `ad607125`) moved two of 118 exports. On RE1 Plumbing the tank went, as intended: elements 124 to 123, the 2 connections not in Revit's export to 0, values Revit does not write 2 to 0, different 0. On Autodesk's Snowdon Towers Structural it left out 91 Generic Models, on a file with no Revit export. In section 3's census the rule never fires on a Generic Model of the 52 models, so nothing supports those 91, and they stay written. Section 2 scores this category on every model with an export, and the rule's false negatives there are 0.

**Limits.** The tank is the one fixture the rule has left out on a model with a Revit export. Reading per type left out 5 more instances than reading per instance in section 3, so a type whose instances differ could be read differently on a model not measured here. Cost: only plumbing fixtures' types are read, the scope of section 4's timing.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=c5bd28e3684feac1e23751f3dfca410d7bb3bd68 -f base=none \
  -f nordic=true -f r2025=true -f holdout=true -f scorers=none \
  -f probe=probe_re363_type_level -f probe_scorer=empty_bodies_vs_ifc
gh workflow run measure.yml --ref main -f ref=af85afada989bc1ea4d734eb868d3fe964606a69 -f base=none \
  -f nordic=true -f r2025=true -f holdout=true -f scorers=none \
  -f probe="probe_re363_plumbing_fixtures probe_re363_empty_bodies" -f probe_scorer="tank_rules_vs_ifc empty_bodies_vs_ifc"
```
