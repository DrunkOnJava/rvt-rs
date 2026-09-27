# Held-out validation: Snowdon Towers Plumbing, HVAC and Electrical

**Date:** 2026-09-27
**Issue:** #408
**Tool:** `tools/re/held_out_vs_vim.py`
**rvt-rs:** `main` at 7df8983 (`rvt-ifc`, default export)

## Why these files

Every number in the README comes from a model the decoder was developed against. These three models were not: no RE report, probe or test reads them. They were downloaded from Autodesk's "Revit Sample Project Files" page (Revit 2024), exported once with `rvt-ifc`, and measured before any change was made for them.

| file | sha256 | size |
|---|---|---:|
| Snowdon Towers Sample Plumbing.rvt | `4f80526bd7823172e236d8c88968210a56c718e69a54a18b7919b3ae5610ae0a` | 42,119,168 |
| Snowdon Towers Sample HVAC.rvt | `585385991b1f8a168881c4bc36546bc90e7bfb427c263d801fe367fa2ebb0fa8` | 22,818,816 |
| Snowdon Towers Sample Electrical.rvt | `e50e1b7121cdcc4ac57cc4777981bbe52012814c9f5f65ea407fcd91b70e1750` | 41,304,064 |

The files are Autodesk's; they are measured locally and never committed.

## Oracle

There is no Revit IFC export of these files. The oracle is `vimaec/vim-hackathon`'s `Snowdon.r2027.vim` (local only, listed in `docs/reference-models.md`), which holds each element's ElementId, BuiltInCategory, Level and UniqueId per source document. It is exported from the **2027 edition** of the sample, not from these 2024 files, so an element that one of them has and the other lacks may be an edition change rather than an error.

## Results

| model | exported | in the VIM, right category | VIM instances with a Level, exported | storey = VIM Level | GlobalId = Revit's (from UniqueId) | bodies |
|---|---:|---:|---:|---:|---:|---|
| Plumbing | 6,253 | 6,213 of 6,213 found | 6,153 of 6,183 | 6,153 of 6,153 | 6,213 of 6,213 | all bounding boxes |
| HVAC | 2,596 | 2,596 | 2,493 of 2,493 | 2,493 of 2,493 | 2,596 of 2,596 | all bounding boxes |
| Electrical | 1,576 | 1,576 | 120 of 120 | 120 of 120 | 1,576 of 1,576 | 1,574 bounding boxes |

- Every exported element the VIM holds is in the category the IFC entity implies: pipes, fittings, ducts, duct fittings, air terminals, plumbing fixtures, lighting fixtures, data devices (`IfcElectricAppliance`), and mechanical equipment, electrical fixtures, electrical equipment and lighting devices as proxies.
- The 30 Plumbing instances the VIM has and rvt-rs does not (20 fittings, 10 pipes) are not declared in the 2024 file's `Global/ElemTable` at all: they were added in the 2027 edition. Every instance both hold is exported.
- The 40 Plumbing elements rvt-rs exports that the VIM lacks (15 pipes, 25 fittings) are declared in the 2024 file. They may have been removed in the 2027 edition. A Revit export of the 2024 file would settle it; none is available.
- The GlobalId check uses the public UniqueId-to-IFC-GUID conversion (the episode GUID with its last four bytes XORed with the element's suffix).
- Electrical's VIM gives most of its elements no Level (face-hosted devices), so only 120 are storey-checked.

## What this does not show

- Geometry: all bodies are bounding boxes; nothing about shape is measured here.
- Names, materials and parameters: the VIM holds them for the 2027 edition only, and are not compared.
- Architectural and structural generalisation: these are MEP models; the architectural and structural numbers stay development-set numbers.
