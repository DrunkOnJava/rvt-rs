# RE-159: a pipe's ConnectionType is not stored on the pipe or its type

**Date:** 2026-10-03
**Issues:** #35; backlog items B31 and B47
**Artefacts:** the four RE1 models (Revit 2025) with Revit's own IFC4 exports. Measured on GitHub's hosted runners with the Measure workflow, run 37145371415.
**Probe:** `examples/probe_re156_value_sources.rs`, extended to list values and to hits inside the element's or its type's own data object.
**Status:** negative.

## 1. The question

Revit's exports of RE1 Mechanical and Plumbing give each of their 69 pipes `Pset_DuctConnection.ConnectionType` (`IfcText`) and `Pset_PipeConnection.ConnectionType` (`IfcLabel`), both the one-item list `Generic`. Where is `Generic`?

## 2. Method

RE-156's probe, with two additions: a list value's first item is looked for like a single value, and each hit is checked against the data objects of the element carrying the value and of its type (RE-153).

## 3. Result

On RE1 Plumbing, for pipe 443719 (type 191045):

- `Generic` occurs 1,250 times as UTF-16 in `Partitions/61`, 4 in `Partitions/62`, once in `Global/Latest`, and 13 times as UTF-8 in `Formats/Latest`.
- None of those hits is inside pipe 443719's own data object or inside its type's.
- The property name `ConnectionType` occurs only in `Formats/Latest`, the schema.

`Generic` is the default name of several things in a Revit project (manufacturers, connection types, materials), so the count says little by itself. The value is not the pipe's or its type's own text. It is most likely the name of a pipe connection-type element that the type's routing preferences point to (B47).

## 4. What this changes

`ConnectionType` is not written. Writing `Generic` on every pipe would hard-code the value Revit happens to use on RE1. The link from a pipe type to its connection-type element is the next probe.

## 5. Reproduce

```text
gh workflow run measure.yml --ref re/re159-connection-type -f base=none -f probe=probe_re156_value_sources
```
