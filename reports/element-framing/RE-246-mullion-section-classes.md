# RE-246 — A mullion type's section class names its shape

**Date:** 2026-10-26
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md) and the 1.0 rule that nothing depends on the language a model was made in; follows RE-243 (which mullions Revit's export draws as an extrusion of their section)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024), Core Interior (Revit 2024), RE1 Architecture (Revit 2025), puzzbobb's R2025 models and Autodesk's 2015 to 2027 sample projects, as Measure fetches them
**Runs:** Measure [40327716858](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40327716858) and [40329833047](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40329833047) (`examples/probe_re246_mullion_families.rs`), [40331557960](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40331557960) (this change against `main`, with the probe, every model) and [40333947780](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40333947780) (both exports of the 2024 rme samples kept)
**Result:** positive: every mullion type's record holds one section object whose class names the section's shape; rvt-rs now tells a rectangular section by it, not by its family's English name. Every export with English family names is byte-identical; Autodesk's MEP samples, whose rectangular mullion family is named in German, now carry the mullions' `NetVolume`.

## 1. The question

RE-243 writes a curtain mullion's `NetVolume`, its box across it times its length, only where its section is a full rectangle, which it took from the mullion's family name: Rectangular Mullion or Quad Corner Mullion. A family's name is in the language Revit was in when the family was made, so a model made in another language would have had no `NetVolume` at all.

## 2. The family record

The probe prints every number, boolean and short string of each mullion type's family record (`SysMullionFamSym.m_familyId`). On the office, the 2024 rac advanced sample and Core Interior the six system families' records (class `Family`) differ only in `m_id`, `m_name` and `m_surrogateId`. Their ids are the same on files started from the same template (8482 Rectangular, 8483 Circular, 18428 L, 18432 V, 18436 Trapezoid and 18440 Quad Corner on the 2024 samples) and others elsewhere (1748 to 1768 on Core Interior), so neither the record nor its id says the kind.

## 3. The type record

Each mullion type's record (`SysMullionFamSym`) holds one section object, whose class is the section's shape:

| class | section | its family on every file probed |
|---|---|---|
| `SysRectMullionProfile` | rectangle | Rectangular Mullion |
| `SysCircMullionProfile` | circle | Circular Mullion |
| `SysLMullionProfile` | L corner | L Corner Mullion |
| `SysVMullionProfile` | V corner | V Corner Mullion |
| `SysTrapMullionProfile` | trapezoid corner | Trapezoid Corner Mullion |
| `SysQuadMullionProfile` | square corner | Quad Corner Mullion |
| `SysCustMullionProfile` | a profile family's | Rectangular Mullion, with a profile family (Core Interior's `Rect_2*10`, `Rect_3*18`) |

A class name is the schema's (`Formats/Latest`), the same whatever language the model is in. A type with a profile family holds `SysCustMullionProfile`, the type RE-243 already left out by its `m_profileId`. On the office as built every one of the 943 mullion type records the probe reads holds `SysRectMullionProfile` (940, family Rectangular Mullion) or `SysQuadMullionProfile` (3, Quad Corner Mullion); at permit 938 and 3; on RE1 Architecture 10 and 0.

Autodesk's MEP sample projects (rme basic and advanced, 2024 to 2027) were made in German: their one mullion family is `Rechteckiger Pfosten`, whose types hold `SysRectMullionProfile` (73) or `SysCustMullionProfile` (184). RE-243's name match gave none of their mullions a `NetVolume`.

## 4. What rvt-rs does

`native_mullion_hosts::mullions_on_plain_hosts` keeps a mullion type only where its record holds a `SysRectMullionProfile` or `SysQuadMullionProfile` (`BOX_SECTION_CLASSES`) besides setting no profile family, and `export_content` no longer reads the family name.

## 5. Measured

Run 40331557960, this change against `main` on all 89 models Measure exports (the reference models, the office, the R2025 models and the 2015 to 2027 samples): 81 exports byte-identical, the office's, Core Interior's and RE1's among them: on each of those the section class keeps exactly the mullions the names did. The 8 that move are the rme basic and advanced samples of 2024 to 2027. On the 2024 rme basic and advanced samples (run 40333947780, both exports kept), 216 of the 1,450 mullions each gains `Qto_MemberBaseQuantities.NetVolume`, and nothing else in either file changes but the owner history's timestamp: the mullions of the `SysRectMullionProfile` types on straight, unedited curtain walls. They have no Revit export to score the values against; the rule that writes them (RE-243's box volume on a plain host) is the one measured equal on all 7,795 mullions of the office as built.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=a815bc85a79ced6379bf742b98129edb3a995e71 -f base=none -f nordic=true -f samples=true \
  -f models="nordic-office-* core-interior re1-architecture sample-2024-*" -f scorers=none \
  -f probe=probe_re246_mullion_families
```
