# RE-171: an element's joins are in its family instances' native connectors

**Issues:** #528. **Backlog:** B54, B86. **Status:** positive on every join of Revit's RE1 exports (Revit 2025), and consistent across Autodesk's MEP samples from 2024 to 2026.

**Evidence:** Measure run 37303312731 (`examples/probe_re171_connector_refs.rs`), on the RE1 models and Autodesk's sample projects. Run 37306537328 is the export (head against main, with `examples/probe_re170_join_list.rs`).

## 1. The question

RE-169 found that a family instance's record holds a `Connector` object per connector, and that each one lists the elements joined to it in `m_arrRefs`. Do those references give Revit's joins, and which field holds the connector's own index?

## 2. The connectors

Read through the native record path, each `Connector` object of a `FamilyInstance` has `m_arrRefs`, `m_mode`, `m_modifiers`, `m_nIndex` and `m_pElement`. Each reference in `m_arrRefs` has `m_id` (the element joined), `m_nIndex` (that element's connector index) and `m_connType`. A reference of type 1 sits beside every physical join. Type 4 names the connector's system, as on the RE1 Plumbing tank's two connectors (RE-169 §5).

The probe takes each connector's own index from each candidate and scores the type-1 references against the `IfcRelConnectsPorts` of Revit's export:

| model | candidate for the own index | joins | Revit's | not Revit's |
|---|---|---:|---:|---:|
| RE1 Mechanical | `m_nIndex` | 73 | **73 of 73** | 0 |
| RE1 Mechanical | position in `m_connPtrArray` + 1 | 73 | 68 | 5 |
| RE1 Mechanical | `m_mode` | 80 | 35 | 45 |
| RE1 Plumbing | `m_nIndex` | 128 | **126 of 126** | 2 |
| RE1 Plumbing | position + 1 | 128 | 124 | 4 |
| RE1 Electrical | — | 0 | 0 of 0 | 0 |

The two joins on Plumbing that Revit's export lacks are those of water closet tank 442378, which Revit's export leaves out (RE-169 §4). Every join has a family instance at one end, because a duct or pipe meets a fitting, terminal or piece of equipment at each end. So the instances' connectors hold every join, ducts' and pipes' included.

## 3. The samples

Autodesk's MEP samples are one project saved from Revit 2024 to 2027. Each release's file decodes 5,598 (`rme_advanced`) or 5,629 (`rme_basic`) family instances with 4,817 or 4,875 connectors, and the export writes 2,609 or 2,652 joins on 2024, 2025 and 2026 alike.

On 2025, where RE-138's and RE-141's list form is also read, the two readers disagree:
- **Duct ends.** 207 of the 265 joins the list form gives on `rme_advanced` that the connectors do not are the same two elements joined at the other end of the duct. The pattern is the same on `rme_basic` (186 of 243). That is the list form's earlier-copy inversion rule, measured on RE1 only, misreading these files.
- **Impossible joins.** The list form joins 11 and 12 duct or pipe ends twice, which cannot happen. The connectors join none twice on any file.

## 4. Result

`native_connectors::family_instance_joins` reads the type-1 references of the MEP categories' family instances, and B86 (#716) writes them as the export's joins wherever the native record path reads the file. RE1 Mechanical writes 73 of 73 and Plumbing 126 of 126.

## 5. Not established

- What `m_mode` is. It is 1 on the RE1 Plumbing tank's piping connectors and on RE1 Electrical panel 428352's electrical connector, and 32 on that panel's conduit connectors.
- Revit 2027's samples decode their connectors (their type-1 references give 4,898 and 4,914 joins), but their export writes no joins. Why was not checked.

## 6. Reproduce

```text
gh workflow run measure.yml -f ref=<branch> -f base=none -f samples=true -f probe=probe_re171_connector_refs
```
