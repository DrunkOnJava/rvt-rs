# RE-170: the joins the list form did not read

**Issues:** #528. **Backlog:** B54, B86. **Status:** positive. All ten unread joins are found, and the finding was superseded by RE-171 for the export.

**Evidence:** Measure run 37301472228 (`examples/probe_re170_unread_joins.rs`), on top of B83 and B86's proxy joins, on the RE1 MEP models (Revit 2025). Run 37303169279 (`examples/probe_re170_join_list.rs`, head against main) gives the joins each sample gained.

## 1. The question

After RE-138 (a duct's or pipe's joins, after its connector manager's anchor) and RE-141 (a fitting's, anywhere, with the fitting first), the export wrote 66 of Revit's 73 joins on RE1 Mechanical and 120 of 126 on Plumbing. Writing joins to proxies (B86) gave 3 of the 5 that joined the air handling unit 427568 to ducts and pipes. Where are the other ten?

## 2. Where they are

The probe exports the model and takes the `IfcRelConnectsPorts` of Revit's export (`MODEL.ifc`) that the export does not write. For each one, it prints every list in RE-138's form (`u32 2`, then two references of `u64` element, `u32` connector index, `u32 1`) that names both ends, in either order, along with its offset from the nearest connector manager anchor before it.

| model | join (Revit's ends) | lists found | first reference | where |
|---|---|---:|---|---|
| RE1 Plumbing | fixture 1 or 2 to pipe 0, six joins | 1 each | the fixture | 1,217 bytes after a pipe's anchor (the later block) |
| RE1 Mechanical | air handling unit 6 and 9 to a pipe and a duct, end 0 | 3 each | the unit | once 1,217 or 1,256 after a curve's anchor, twice before any anchor in its partition |
| RE1 Mechanical | air terminal 1 to duct fitting 1, two joins | 3 or 4 each | the terminal | before any anchor, and 287,770 and 564,251 bytes after one |

- **Every unread join is a list with the terminal or equipment first.** A duct's or pipe's own lists put the duct or pipe first, and RE-141's scan required a fitting first.
- **Both indices are stored as Revit numbers them**, the pipe's end 0 in the later block included, and every copy of a list agrees.

## 3. Why the export does not use it

Accepting lists with any terminal or equipment first, found anywhere and with connector indices up to 31, wrote all 73 and 126 joins on RE1. It also added 252 and 249 joins to Autodesk's 2025 MEP samples (run 37303169279), which no export of Revit's can score. RE-171 reads the same joins from decoded records instead and shows the list form misreads the samples, so B86 reads the records.

## 4. Reproduce

```text
gh workflow run measure.yml -f ref=<branch> -f base=none -f probe=probe_re170_unread_joins
gh workflow run measure.yml -f ref=<branch> -f base=<base branch with the probe> -f samples=true -f probe=probe_re170_join_list
```
