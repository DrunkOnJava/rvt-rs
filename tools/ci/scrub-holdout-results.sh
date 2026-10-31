#!/usr/bin/env bash
# Reduce the Measure results of each model fetched without a licence to
# redistribute it, the held-out models (research/holdout-models.tsv, RE-252)
# and BIMcollab's example model (research/bimcollab-models.tsv, RE-198), to
# summary counts before they are uploaded.
#
# Usage:
#   tools/ci/scrub-holdout-results.sh RESULTS_DIR
#
# A public repository's artifacts can be downloaded by anyone, so nothing taken
# from those models per element may be uploaded (Griffin, 2026-10-26). For
# each RESULTS_DIR/holdout-*/ and RESULTS_DIR/bimcollab-*/ this keeps
# counts.txt, made of the summary lines of its probe scores (the wall length
# rule, with the walls it is wrong on as a count, the export's version, the
# census of its wall representations, the hours between its save and its
# export and the railing rules' counts), the count lines a probe prints for
# such a model (`type refs:` and `type copies:`, RE-253), the `native framing:`
# lines of probe_re280_native_framing_pre2023 (RE-280: counts, schema class and
# field names and normalised error text, in a restricted alphabet; its JSON
# example lines, which carry ElementIds, are dropped), the `native readers:`
# count lines of the RE-283 probes (the same alphabet; their `native readers
# example:` lines name ElementIds and are dropped) and
# of its quantity census (the totals and each quantity's counts, the walls it
# counts apart for their two lengths (RE-252), for each value that
# differs only the quantity and rvt-rs's value over Revit's, and for the
# quantities rvt-rs writes that Revit's export does not only each quantity's
# name and count), of
# its names census (counts by category, entity and agreement) and of its layer
# set census (its counts, not its worst elements), of its identity censuses
# (Griffin, 2026-10-29: counts only, never an ElementId or a name): the
# class-set totals and each class's revit / same / other / absent counts and
# the extra elements' count by class, the GlobalId agreement counts, and the
# storey verdict counts by kind, and
# the `native levels:` lines of tools/re/native_levels_vs_ifc.py (RE-281:
# counts, the same restricted alphabet), the totals and per property counts of
# its property set census (standard Pset_ and Qto_ names only; RE-281), and
# ifc.sha256, the hash of rvt-rs's own export; every other file is deleted.
set -euo pipefail

RESULTS="${1:?usage: scrub-holdout-results.sh RESULTS_DIR}"
for dir in "$RESULTS"/holdout-*/ "$RESULTS"/bimcollab-*/; do
  [ -d "$dir" ] || continue
  {
    for f in "$dir"probe_score*.txt; do
      [ -f "$f" ] || continue
      grep -E "^(rule [a-z ,=]+: [0-9]+ walls, span is Revit's Length on [0-9]+, not on \[[0-9, ]*\]|walls with Revit's Length and a GLine: [0-9]+|a choice at each end gives the Length: [0-9]+|walls by their number of Qto_WallBaseQuantities sets: [][0-9(), ]+|walls Revit gives a Width the partition reader gives no centreline: [0-9]+; with layers [0-9]+, Width = their total on [0-9]+|export time vs save: (export less save in hours: -?[0-9]+\.[0-9]|not read \((no save time|no export time)\))|export wall types not in the file: [0-9]+ of [0-9]+; walls of them: [0-9]+|wall (boxes|faces) vs Revit Width: walls [0-9]+; box side equal [0-9]+; thinner [0-9]+( \(ratios( [0-9]+\.[0-9]+)+\))?; thicker [0-9]+|railings with Revit's Length: [0-9]+(; [a-z ]+ [0-9]+)*|railings a stair holds: [0-9]+; the (stair|run count) rule gives Revit's Length on [0-9]+|railings a stair holds whose path is one line along its runs: [0-9]+; the one line rule gives Revit's Length on [0-9]+|exit [0-9]+)$" "$f" \
        | awk '{ if (match($0, /not on \[[0-9, ]*\]$/)) { ids = substr($0, RSTART + 8, RLENGTH - 9); n = ids == "" ? 0 : split(ids, a, ","); $0 = substr($0, 1, RSTART - 1) "not on " n " walls" } print }' || true
    done
    for f in "$dir"probe_score*.txt; do
      [ -f "$f" ] || continue
      grep -E '^native levels: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
    done
    for f in "$dir"psets_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^(Revit values on objects rvt-rs writes: [0-9]+; equal: [0-9]+; different: [0-9]+; missing: [0-9]+; rvt-rs values Revit does not write: [0-9]+|  (Pset|Qto)_[A-Za-z0-9]+\.[A-Za-z0-9]+: [0-9]+, equal [0-9]+, different [0-9]+, missing [0-9]+|  (Pset|Qto)_[A-Za-z0-9]+\.[A-Za-z0-9]+: [0-9]+, [0-9]+|  (Pset|Qto)_[A-Za-z0-9]+\.[A-Za-z0-9]+: [0-9]+|IFC4 properties IFC2X3's definitions do not have, which Revit's IFC2X3 export cannot write, not scored: [0-9]+|rvt-rs values Revit does not write, on objects Revit writes and on others:|IFC2X3 (lengths IFC4 holds as quantities|values also in an IFC4 set and scored there), not scored here: [0-9]+|IFC2X3 values no IFC4 set Revit's export writes holds, in rvt-rs's own set, not scored here: [0-9]+)$" "$f" | sed 's/^/psets: /' || true
    done
    for f in "$dir"probe.txt "$dir"probe-*.txt; do
      [ -f "$f" ] || continue
      grep -E '^type (refs|copies): [A-Za-z ,;:()-]+[0-9][0-9A-Za-z ,;:()-]*$' "$f" || true
      grep -E '^native framing: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
      grep -E '^native readers: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
    done
    for f in "$dir"qtos_vs_ifc*.txt; do
      [ -f "$f" ] || continue
      grep -E "^(Revit quantities on objects rvt-rs writes: [0-9]+; equal: [0-9]+; different: [0-9]+; missing: [0-9]+; rvt-rs quantities Revit does not write: [0-9]+|  Qto_[A-Za-z]+\.[A-Za-z]+: [0-9]+, equal [0-9]+, different [0-9]+, missing [0-9]+|walls whose curve and its cut-back curve differ, either of which Revit's export may write as their Length, counted apart \(RE-252\): [0-9]+|walls whose Length is rvt-rs's curve's: [0-9]+; Revit's Axis's: [0-9]+; neither: [0-9]+( \(none counted apart\))?|walls the two exports give different types, their differing Width counted apart \(RE-253\): [0-9]+|walls the two exports give different types whose record does not confirm rvt-rs's, their Width scored \(RE-253\): [0-9]+|exit [0-9]+)$" "$f" || true
      awk '/^largest differences/ { on = 1; next }
           on && $2 ~ /^Qto_[A-Za-z]+\.[A-Za-z]+:$/ && $6 + 0 != 0 {
             q = $2; sub(/:$/, "", q); a = $4; sub(/,$/, "", a)
             printf "different %s: rvt-rs over Revit %.6f\n", q, a / $6
           }' "$f"
      # How many values of each quantity rvt-rs writes that Revit's export does
      # not: a count per quantity name, never the objects the scorer lists under it.
      awk '/^rvt-rs quantities Revit does not write:/ { on = 1; next }
           on && /^  Qto_[A-Za-z]+\.[A-Za-z]+: [0-9]+$/ { sub(/^  /, ""); print "extra " $0; next }
           on && !/^ / { on = 0 }' "$f"
    done
    for f in "$dir"names_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E '^ +[0-9]+  (system family|family instance|[a-z ]+) \| Ifc[A-Za-z]+ \| Name (equal|differs) \| ObjectType (equal|differs)$' "$f" | sed 's/^/names: /' || true
    done
    for f in "$dir"layer_sets_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^( +[0-9]+  [a-z][a-z ]*(: [A-Za-z',: ]+| = Revit's Family:Type)?|  [a-z ]+ within [0-9.]+ ft of Revit's: [0-9]+ of [0-9]+)$" "$f" | sed 's/^/layer sets: /' || true
    done
    for f in "$dir"class_sets_vs_ifc.txt; do
      [ -f "$f" ] || continue
      awk '/^extra in rvt-rs.s export, by class:$/ { extra = 1; next }
           /^(Revit.s|rvt-rs.s) export: [0-9]+ paired elements and spaces, [0-9]+ untagged and [0-9]+ repeated Tags left out$/ { print "class sets: " $0; next }
           !extra && /^Ifc[A-Za-z0-9]+ +[0-9]+ +[0-9]+ +[0-9]+ +[0-9]+  (Ifc[A-Za-z0-9]+ [0-9]+(, Ifc[A-Za-z0-9]+ [0-9]+)*)?$/ { print "class sets: " $0; next }
           extra && /^Ifc[A-Za-z0-9]+ +[0-9]+$/ { print "class sets extra: " $0; next }
           { extra = 0 }' "$f"
    done
    for f in "$dir"global_ids_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^(elements: \{((\"not in Revit's export\"|\"GlobalId = Revit's\"|'differs'): [0-9]+(, )?)*\}|storeys: [0-9]+ of [0-9]+ equal Revit's|storeys whose GlobalId is one of Revit's: [0-9]+ of [0-9]+|(IfcProject|IfcSite|IfcBuilding): [0-9]+ of [0-9]+ equal Revit's|spaces: [0-9]+ of [0-9]+ equal one of Revit's)$" "$f" | sed 's/^/global ids: /' || true
    done
    for f in "$dir"storeys_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^([0-9]+ elements, [0-9]+ not on a storey|against Revit's export:| +[0-9]+  Ifc[A-Za-z0-9]+| +[0-9]+  (same storey as Revit's|on no storey; Revit's is (a storey|another container)|on a storey; Revit contains it in <Ifc[A-Za-z0-9]+>|a different storey from Revit's)|  different, by type: Ifc[A-Za-z0-9]+ [0-9]+(, Ifc[A-Za-z0-9]+ [0-9]+)*)$" "$f" | sed 's/^/storeys: /' || true
    done
  } > "$RUNNER_TEMP/counts.txt"
  find "$dir" -mindepth 1 ! -name ifc.sha256 -delete
  mv "$RUNNER_TEMP/counts.txt" "${dir}counts.txt"
  echo "$(basename "$dir"): $(wc -l < "${dir}counts.txt" | tr -d ' ') count lines kept"
done
