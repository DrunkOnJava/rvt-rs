# RE-226 — Curtain mullions' hosts and spans

**Date:** 2026-10-25
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-193 §6 and RE-106
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, with Revit's IFC4 exports) and RE1 Architecture (Revit 2025, with Revit's IFC2X3 export), as Measure fetches them
**Runs:** Measure [40228368637](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40228368637) and [40229976360](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40229976360) (`examples/probe_re226_mullion_hosts.rs` with `tools/re/member_hosts_vs_ifc.py`, `keep_ifc`), and [40231117081](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40231117081) (this change against `main`, `psets_vs_ifc`)
**Result:** negative for `IsExternal`, positive for `Span`.

## 1. What was missing

On the BIM4LCA office, Revit's export writes 25,598 property values rvt-rs did not, and 20,006 of them are two properties of the 10,003 curtain mullions: `Pset_MemberCommon.IsExternal` (false 9,505 times, true 498) and `Pset_MemberCommon.Span` (`psets_vs_ifc`, which lists the values Revit writes most often since #992).

## 2. `IsExternal`: the host rule, measured negative

revit-ifc's `CategoryUtil.IsElementExternal` (CLEANROOM.md source 5) gives a family instance with neither an `IsExternal` parameter nor a type Function its host's value, and false where it has no host. RE-193 §6 took the curtain wall that holds a mullion and found mullions of one type on external curtain walls not external 8,460 times and external 397 times. Every mullion's and panel's own `FamilyInstance` record does name a host, `m_hostId`: its curtain wall, an `SWall` (9,739) or an `ArcWall` (264). Giving each the `IsExternal` Revit's export writes on that wall:

| file | members right | plates right |
|---|---:|---:|
| office as built | 1,424 of 10,003 | 227 of 260 |
| office at permit | 1,454 of 10,031 | 237 of 269 |
| RE1 Architecture | 10 of 10 | 2 of 2 |

RE1 agrees only because every value there is false. The host does not decide a mullion's `IsExternal`; rvt-rs still writes none on mullions and panels.

## 3. `Span`: a mullion's length

revit-ifc's `SpanCalculator` takes `Span` from a parameter of that name where one is set, else from the length of the body's extrusion. Revit's `Span` is the depth of its own mullion extrusion on 9,796 of the office's 10,003 mullions. Against the box rvt-rs draws:

| measure of the rvt-rs box | equal to Revit's `Span`, as built |
|---|---:|
| longest side | 9,635 |
| extrusion along the mullion's own Z (RE-106's oriented box, 325 mullions) | 325 of 325 |

The longest side misses the mullions shorter than their own section (Revit's `Span` of 0.328 ft against a 0.4921 ft section). A mullion runs along its own Z axis: RE-106's axes give that for a mullion turned off the model's axes, and for one on them the model axis its transform's Z lies on.

## 4. What rvt-rs does

`attach_curtain_axes` records, for a mullion on the model's axes, the model axis it runs along (`CURTAIN_MULLION_ALONG_FIELD`); a turned or tilted one keeps its three axes. `add_mullion_span` writes `Pset_MemberCommon.Span` as the record box's extent along that axis, or the oriented box's extent along the mullion's Z.

## 5. Measured

`psets_vs_ifc`, this change against `main` (run 40231117081):

| file | `Span` equal | different | missing | all values equal |
|---|---:|---:|---:|---|
| office as built | 9,984 of 10,003 | 1 | 18 | 32,167 → 42,151 |
| office at permit | 10,012 of 10,031 | 1 | 18 | 32,339 → 42,351 |

rvt-rs's length is Revit's `Span` on 9,984 mullions, more than the 9,796 whose `Span` is Revit's own extrusion depth. Core Interior, Einhoven and RE1 Electrical, Mechanical and Plumbing export byte for byte as before. RE1 Architecture's 10 mullions now carry a `Span` that Revit's IFC2X3 export of that model does not write; rvt-rs writes IFC4, where Revit's export of the office does.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref feat/mullion-span -f ref=feat/mullion-span -f base=none \
  -f nordic=true -f keep_ifc=true -f probe=probe_re226_mullion_hosts -f probe_scorer=member_hosts_vs_ifc
gh workflow run measure.yml --ref main -f ref=feat/mullion-span -f base=main -f nordic=true
```
