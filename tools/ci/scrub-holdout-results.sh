#!/usr/bin/env bash
# Reduce the Measure results of each model fetched without a licence to
# redistribute it, the held-out models (research/holdout-models.tsv, RE-252),
# BIMcollab's example model (research/bimcollab-models.tsv, RE-198) and
# Autodesk's rac_advanced sample scored against an IFC2X3 export that carries
# no licence (research/rac-advanced-export.tsv, #1354; the main session,
# 2026-11-04, after Griffin's precedent for BIMcollab), to summary counts
# before they are uploaded.
#
# Usage:
#   tools/ci/scrub-holdout-results.sh RESULTS_DIR
#
# A public repository's artifacts can be downloaded by anyone, so nothing taken
# from those models per element may be uploaded (Griffin, 2026-10-26). For
# each RESULTS_DIR/holdout-*/, RESULTS_DIR/bimcollab-*/ and RESULTS_DIR/racadv-*/
# this keeps
# counts.txt, made of the summary lines of its probe scores (the wall length
# rule, with the walls it is wrong on as a count, the export's version, the
# census of its wall representations, the hours between its save and its
# export and the railing rules' counts), the count lines a probe prints for
# such a model (`type refs:` and `type copies:`, RE-253), the `native framing:`
# lines of probe_re280_native_framing_pre2023 (RE-280: counts, schema class and
# field names and normalised error text, in a restricted alphabet; its JSON
# example lines, which carry ElementIds, are dropped), the `saved graphics
# pre2023:` count lines of probe_re342_saved_graphics_pre2023 (RE-342: counts,
# schema class and field names and normalised error text, in the same
# alphabet; it prints no ElementId), the `native readers:`
# count lines of the RE-283 to RE-285 probes and scorers (the same alphabet;
# their `native readers example:` lines name ElementIds and are dropped), the
# `previous lines:` count line of probe_re360_wall_location_curve (RE-360: how
# many wall records hold a previous location line, how many of those it is offset
# from the location curve and how many walls the previous-line rule reads, numbers
# beside fixed words; the probe's JSON lines, which carry ElementIds, are dropped), the
# `void rule:` count line of probe_re355_void_loop_directions (RE-355: how many
# floor voids' first lines run against or along their loop's walk and how many
# tags that gives, as numbers beside fixed words, never an id), the
# `census stages:` count lines of probe_re350_category_stages and the
# `category census` lines of tools/re/category_census_vs_ifc.py (RE-350: for
# each built-in category the counts of ids by stage and reason, the export
# classes they land on and the export's counts by class, beside the category's
# built-in name and id and an IFC class name, never an ElementId or a name
# from the model; the scorer's table rows, which carry the export's category
# name, and its `not in the export` lines are dropped), the `re363` count
# lines of tools/re/tank_rules_vs_ifc.py (RE-363: for each candidate rule the
# counts of plumbing fixtures it would drop, in and not in Revit's export,
# beside the rule's name and the export's schema, never an ElementId or a name
# from the model; its listing lines are dropped), the
# `door flips:` count lines of tools/re/door_flips_vs_ifc.py (RE-343: counts of
# doors and windows by flag state and by how their placements compare; its
# `field ... values` lines only with values true, false, null or under 100; its
# `row` and `sample` lines name ElementIds and are dropped) and
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
# the `mcp answers:` count lines of tools/re/mcp_answers_vs_ifc.py (#1327: the
# MCP server's answers against the export and Revit's, as counts and verdicts
# beside an IFC entity name, never an element), the `native levels:` lines of
# tools/re/native_levels_vs_ifc.py (RE-281:
# counts, the same restricted alphabet), the totals and per property counts of
# its property set census (standard Pset_ and Qto_ names only; RE-281),
# the `door axes:` count lines of tools/re/door_axes_vs_ifc.py (RE-347: how many
# doors and windows have rvt-rs's local axes the same as Revit's, a half turn or
# a quarter turn from them, and how many placed by the rule differ, as numbers
# beside fixed words), the `space kinds:` count lines of tools/re/space_kinds_vs_ifc.py (#1437: how
# many `IfcSpace`s hold each `SpaceKind`, how many hold none, and how many
# disagree with their `SpaceOutlineSource`, as numbers beside fixed words, never
# a GlobalId; its `exit N` line stays as `space kinds: exit N`, so the run's
# total tells a crash from a model that was not checked),
# the `space profiles:` count lines of tools/re/space_profiles_vs_ifc.py
# (RE-313: how many spaces pair, and their area and outline within a tolerance,
# never a GlobalId), the `space overlaps:` count lines of
# tools/re/space_overlaps_vs_ifc.py (RE-316: how many rooms of each source have
# the area of Revit's upward faces, floor or every face, never a GlobalId), the
# totals and per-class rows of mesh_bodies_vs_ifc.py, box_bodies_vs_ifc.py and
# saved_meshes_vs_ifc.py (RE-329: numbers beside an IFC class name, never a
# family, type or element), with the `fine` rows the two body scorers print under
# RVT_SCORER_FINE_MESH (RE-331: the same), the last column those two print when the
# fine tessellation decides some bodies and the line that counts them (RE-331
# section 8, #1365: the same), the one `shell readings:` line mesh_bodies_vs_ifc.py
# prints (#1658: how many volume outliers it read and how they come out, as numbers
# beside fixed words, never a Tag or a type), the counts of the IFC4 schema's findings on
# the export (`schema validate:` and `schema rules:` lines, #1430: a number, and
# for a rule finding the entity and rule that name it, never its message), the
# `projected graphs:` count lines of probe_1335_projected_graphs (#1335: counts
# of records decoded and of differences, a verdict word for the catalog, schema
# class names and the field names of the readers' lists, never a file name or an
# ElementId) with its `probe: exit N` line, the `wall rules:` count lines of
# tools/re/wall_rule_counts.py (RE-348, RE-349: how many walls take the wall base
# and end rules and why the others do not, as numbers beside fixed words and
# reason names, never an ElementId), the `wall heights:` count lines of
# tools/re/wall_heights_vs_ifc.py (RE-257, RE-351: how many walls stand at
# Revit's base and top, and how many floors, roofs and footings cut them, as
# numbers beside the scorer's fixed words, never a Tag, a height or the files'
# offset), and
# ifc.sha256, the hash of rvt-rs's own export; every other file is deleted.
set -euo pipefail

RESULTS="${1:?usage: scrub-holdout-results.sh RESULTS_DIR}"
for dir in "$RESULTS"/holdout-*/ "$RESULTS"/bimcollab-*/ "$RESULTS"/racadv-*/; do
  [ -d "$dir" ] || continue
  {
    # Says which census lines this scrub keeps: tools/re/counted_apart.py reads a count file without it as one made
    # before the counted-apart lines were kept, and shows those counts as not kept, not as zero.
    echo "counted apart: lines kept 1"
    for f in "$dir"probe_score*.txt; do
      [ -f "$f" ] || continue
      grep -E "^(rule [a-z ,=]+: [0-9]+ walls, span is Revit's Length on [0-9]+, not on \[[0-9, ]*\]|walls with Revit's Length and a GLine: [0-9]+|a choice at each end gives the Length: [0-9]+|walls by their number of Qto_WallBaseQuantities sets: [][0-9(), ]+|walls Revit gives a Width the partition reader gives no centreline: [0-9]+; with layers [0-9]+, Width = their total on [0-9]+|export time vs save: (export less save in hours: -?[0-9]+\.[0-9]|not read \((no save time|no export time)\))|export wall types not in the file: [0-9]+ of [0-9]+; walls of them: [0-9]+|wall (boxes|faces) vs Revit Width: walls [0-9]+; box side equal [0-9]+; thinner [0-9]+( \(ratios( [0-9]+\.[0-9]+)+\))?; thicker [0-9]+|railings with Revit's Length: [0-9]+(; [a-z ]+ [0-9]+)*|railings a stair holds: [0-9]+; the (stair|run count) rule gives Revit's Length on [0-9]+|railings a stair holds whose path is one line along its runs: [0-9]+; the one line rule gives Revit's Length on [0-9]+|exit [0-9]+)$" "$f" \
        | awk '{ if (match($0, /not on \[[0-9, ]*\]$/)) { ids = substr($0, RSTART + 8, RLENGTH - 9); n = ids == "" ? 0 : split(ids, a, ","); $0 = substr($0, 1, RSTART - 1) "not on " n " walls" } print }' || true
    done
    for f in "$dir"probe_score*.txt; do
      [ -f "$f" ] || continue
      grep -E '^native levels: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
      # tools/re/beam_lines_vs_ifc.py (#1697): counts beside fixed words and the
      # export's units, never an ElementId; its `--list` lines are dropped.
      grep -E "^beam lines: [A-Za-z0-9 _:.,;()=#<>/+'-]+$" "$f" || true
      grep -E '^native readers: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
      grep -E '^category census[A-Za-z ]*: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
      grep -E '^re363 [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
      grep -E '^door flips: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" | grep -v -E '^door flips: field ' || true
      grep -E '^door flips: field [A-Za-z0-9_]+ values ((true|false|null|[0-9]{1,2})=[0-9]+(, )?)+$' "$f" || true
      # tools/re/panel_material_sources_vs_ifc.py (RE-377, #1593): counts beside
      # fixed words, schema class and field names and a digest of an entry's other
      # fields, never a material name or an ElementId. Its `detail` lines show the
      # bytes behind a digest and are dropped.
      grep -E "^panel material sources: (plates in Revit's export [0-9]+, with a material [0-9]+, panels the probe lists [0-9]+, plates scored [0-9]+|records selected [0-9?]+, read [0-9?]+, holding a material [0-9?]+|plates with a source giving exactly Revit's material [0-9]+ of [0-9]+)$" "$f" || true
      grep -E "^panel material references: (Revit's plate materials [0-9]+, named in the file [0-9]+|of those named, [0-9]+ held by no record's body|class [A-Za-z0-9_]+ holds one in [0-9]+ records|Curtain Panels category styles holding a material [0-9]+, naming [0-9]+ materials; Revit's plate material is one of them on [0-9]+ of [0-9]+ plates)$" "$f" || true
      grep -E '^panel material source: (instance|type|ref) [A-Za-z0-9_]+ [][A-Za-z0-9_.]+ ([0-9a-f]{8}|-) present [0-9]+, equal [0-9]+, contains [0-9]+, different [0-9]+$' "$f" || true
    done
    # What Revit's material for each element is against rvt-rs's (RE-82): counts
    # and IFC entity names only.
    for f in "$dir"element_materials_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^(elements Revit gives materials: [0-9]+|of those, rvt-rs gives materials: [0-9]+, the same set: [0-9]+)$" "$f" | sed 's/^/materials: /' || true
      grep -E "^(different sets by|no materials from rvt-rs, by) Revit entity: \{((')?Ifc[A-Za-z0-9]+(')?: [0-9]+(, )?)*\}$" "$f" | sed 's/^/materials: /' || true
    done
    for f in "$dir"psets_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^(Revit values on objects rvt-rs writes: [0-9]+; equal: [0-9]+; different: [0-9]+; missing: [0-9]+; rvt-rs values Revit does not write: [0-9]+|  (Pset|Qto)_[A-Za-z0-9]+\.[A-Za-z0-9]+: [0-9]+, equal [0-9]+, different [0-9]+, missing [0-9]+|  (Pset|Qto)_[A-Za-z0-9]+\.[A-Za-z0-9]+: [0-9]+, [0-9]+|  (Pset|Qto)_[A-Za-z0-9]+\.[A-Za-z0-9]+: [0-9]+|IFC4 properties IFC2X3's definitions do not have, which Revit's IFC2X3 export cannot write, not scored: [0-9]+|rvt-rs values Revit does not write, on objects Revit writes and on others:|values on a key that more than one object of Revit's export holds, scored against the object held, in the totals above \(#1576\): [0-9]+ on [0-9]+ such keys; equal [0-9]+, different [0-9]+, missing [0-9]+|values on a key that more than one object of Revit's export holds, whose objects hold different values, counted apart \(#1576\): [0-9]+ on [0-9]+ such keys|  on a repeated key, (Pset|Qto)_[A-Za-z0-9]+\.[A-Za-z0-9]+: [0-9]+ counted apart|IFC2X3 (lengths IFC4 holds as quantities|values also in an IFC4 set and scored there), not scored here: [0-9]+|IFC2X3 values no IFC4 set Revit's export writes holds, in rvt-rs's own set, not scored here: [0-9]+|Revit values known to be wrong, left out \(RE-235 §5\): (Pset|Qto)_[A-Za-z0-9]+\.[A-Za-z0-9]+ [0-9]+|Revit values known to be wrong, left out \(RE-235 §5, RE-269\): (Pset|Qto)_[A-Za-z0-9]+\.[A-Za-z0-9]+ on [a-z ]+ [0-9]+|Revit values the project's own property set file places, out of scope \(RE-239\), not scored: [0-9]+|Revit values on openings rvt-rs does not write \(a door or wall cut in a further host, a curtain wall's or cabinet's cut\), not scored: [0-9]+|Revit's export writes no standard property set \(its setup, RE-239 §5\); rvt-rs values not scored: [0-9]+|rvt-rs values on openings Revit's export of this file lacks but its export of a sibling file holds, counted apart \(a pair inconsistency, RE-338\): [0-9]+ on [0-9]+ openings)$" "$f" | sed 's/^/psets: /' || true
    done
    for f in "$dir"probe.txt "$dir"probe-*.txt; do
      [ -f "$f" ] || continue
      grep -E '^type (refs|copies): [A-Za-z ,;:()-]+[0-9][0-9A-Za-z ,;:()-]*$' "$f" || true
      grep -E '^native framing: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
      grep -E '^native readers: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
      grep -E '^saved graphics pre2023: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
      grep -E '^census stages: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
      grep -E '^previous lines: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
      # probe_1335_projected_graphs (#1335): counts, verdict words, schema class
      # names (a class name is the schema's, never the model's) and the field
      # names of the readers' lists, as numbers beside fixed words. Its `file`
      # line names the model's file and its `native:` line may carry an error
      # with an ElementId; both are dropped, and `exit N` stays.
      grep -E '^(global-catalog [a-z-]+|compared records [0-9]+ decodes [0-9]+ full-ok [0-9]+ full-refused [0-9]+ mismatches [0-9]+ budget-stopped (true|false)|keep [A-Za-z0-9_,]* compared [0-9]+ mismatches [0-9]+|refused [A-Za-z0-9_]* [0-9]+|binding-context family-categories [0-9]+ symbol-families [0-9]+ same (true|false) ingest-refusals [0-9]+ same-refusals (true|false)|wall-facts compared [0-9]+ same (true|false)|plan-cache compared [0-9]+ mismatches [0-9]+|plan-cache limited compared [0-9]+ mismatches [0-9]+|plan-cache size peak-entries [0-9]+ peak-cost [0-9]+ clears [0-9]+|mismatch [a-z-]+ [A-Za-z0-9_]*)$' "$f" | sed 's/^/projected graphs: /' || true
      grep -E '^exit [0-9]+$' "$f" | sed 's/^/probe: /' || true
      grep -E '^void rule: [A-Za-z0-9 ,]+$' "$f" || true
    done
    for f in "$dir"wall_rules.txt; do
      [ -f "$f" ] || continue
      grep -E '^wall rules: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
    done
    for f in "$dir"qtos_vs_ifc*.txt; do
      [ -f "$f" ] || continue
      grep -E "^(Revit quantities on objects rvt-rs writes: [0-9]+; equal: [0-9]+; different: [0-9]+; missing: [0-9]+; rvt-rs quantities Revit does not write: [0-9]+|  Qto_[A-Za-z]+\.[A-Za-z]+: [0-9]+, equal [0-9]+, different [0-9]+, missing [0-9]+|walls whose curve and its cut-back curve differ, either of which Revit's export may write as their Length, counted apart \(RE-252\): [0-9]+|walls whose Length is rvt-rs's curve's: [0-9]+; Revit's Axis's: [0-9]+; neither: [0-9]+( \(none counted apart\))?|walls the two exports give different types, their differing Width counted apart \(RE-253\): [0-9]+|walls the two exports give different types, their other differing wall quantities counted apart \(RE-253, RE-388\): [0-9]+|walls the two exports give different types whose record does not confirm rvt-rs's, their Width scored \(RE-253\): [0-9]+|values on a key that more than one object of Revit's export holds, counted apart \(#1576\): [0-9]+ of Revit's on [0-9]+ such keys, and [0-9]+ of rvt-rs's with none in the object held|  on a repeated key, Qto_[A-Za-z]+\.[A-Za-z]+: [0-9]+; against the object held: equal [0-9]+, different [0-9]+, missing [0-9]+|Revit's export writes no base quantities \(its setup's Export base quantities off, RE-242\): rvt-rs writes [0-9]+, which rvt-ifc --no-base-quantities leaves out|Revit values known to be wrong, left out \(RE-235 §5\): Qto_[A-Za-z]+\.[A-Za-z]+ [0-9]+|Revit quantities on openings rvt-rs does not write \(a door or wall cut in a further host, a curtain wall's or cabinet's cut, #1165\), not scored: [0-9]+|quantities with their own unit, not scored: rvt-rs [0-9]+, Revit [0-9]+|exit [0-9]+)$" "$f" || true
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
    # tools/re/predefined_types_vs_ifc.py (RE-419): each class's summary line, counts and
    # enumerator names beside an IFC class name, never a Tag; the lines under it that name
    # a Tag are dropped.
    for f in "$dir"predefined_types_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^Ifc[A-Za-z0-9]+: [0-9]+ in both; same [0-9]+, differs [0-9]+, unset in rvt-rs [0-9]+, NOTDEFINED where Revit writes none [0-9]+; Revit's: \{(('[A-Z_0-9-]+': [0-9]+)(, )?)*\}$" "$f" | sed 's/^/predefined types: /' || true
      # And the tally of each type class's PredefinedType, counts beside enumerator names.
      grep -E "^Ifc[A-Za-z0-9]+Type PredefinedType: rvt-rs \{(('[A-Z_0-9-]+': [0-9]+)(, )?)*\}, Revit \{(('[A-Z_0-9-]+': [0-9]+)(, )?)*\}$" "$f" | sed 's/^/predefined types: /' || true
    done
    for f in "$dir"names_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E '^ +[0-9]+  (system family|family instance|[a-z ]+) \| Ifc[A-Za-z]+ \| Name (equal|differs) \| ObjectType (equal|differs)$' "$f" | sed 's/^/names: /' || true
      # How an ObjectType that differs relates to Revit's (#1472), in fixed words.
      grep -E "^ +[0-9]+  (system family|family instance|[a-z ]+) \\| Ifc[A-Za-z]+ \\| ObjectType pattern: (rvt-rs writes none|Revit writes none|case only|spacing or punctuation only|Revit's is the type part of rvt-rs's|rvt-rs's is the type part of Revit's|Revit's extends rvt-rs's|rvt-rs's extends Revit's|same family, other type|other family, same type|other)$" "$f" | sed 's/^/names: /' || true
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
    # The local axes of the doors and windows against Revit's (RE-347,
    # tools/re/door_axes_vs_ifc.py): counts beside fixed words. Its `exit N`
    # line stays, as a `door axes:` line, so a crash is told from a model
    # that was not checked, and a rule-placed element that differs (exit 1).
    for f in "$dir"door_axes_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E '^door axes: [A-Za-z0-9 _:.,;()=#<>/+-]+$' "$f" || true
      grep -E '^exit [0-9]+$' "$f" | sed -E 's/^exit ([0-9]+)$/door axes: exit \1/' || true
    done
    for f in "$dir"space_profiles_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E '^space profiles: [A-Za-z0-9 %.,:()-]+$' "$f" || true
    done
    for f in "$dir"space_kinds_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E '^space kinds: [A-Za-z0-9 _:.,;()=%/+-]+$' "$f" || true
      grep -E '^exit [0-9]+$' "$f" | sed -E 's/^exit ([0-9]+)$/space kinds: exit \1/' || true
    done
    for f in "$dir"space_overlaps_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^space overlaps: (upward|downward|every) faces?, [a-z ]+: [0-9]+ paired rooms, area within 0\.1% of Revit's on [0-9]+$" "$f" || true
    done
    # The saved-mesh bodies (mesh_bodies_vs_ifc), the record boxes left
    # (box_bodies_vs_ifc) and the per-class world-box agreement
    # (saved_meshes_vs_ifc): their totals and class rows, which are numbers
    # beside an IFC class name. The rows under a class, which name a family and
    # type, are dropped (RE-329).
    for f in "$dir"mesh_bodies_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^(saved_graphics_mesh bodies scored: [0-9]+; in Revit's export [0-9]+|Ifc[A-Za-z0-9]+: [0-9]+ bodies, every [0-9]+th scored|Ifc[A-Za-z0-9]+ +[0-9]+( +[0-9]+){5} +([0-9]+\.[0-9]+|nan|-)( +[0-9]+)?|decided by the fine tessellation \(both files write true curves\): [0-9]+ of [0-9]+ bodies)$" "$f" | sed 's/^/mesh bodies: /' || true
      grep -E "^(fine tessellation: IfcOpenShell mesher-linear-deflection [0-9.e-]+ m \(1e-5 ft\) and mesher-angular-deflection [0-9.]+ rad, both files; [0-9]+ of [0-9]+ bodies meshed in both, [0-9]+ of them meshed differently|fine (Ifc[A-Za-z0-9]+|total) +[0-9]+( +[0-9]+){8}  ([0-9]+\.[0-9]+|nan|inf) [0-9]+\.[0-9]+|fine change (Ifc[A-Za-z0-9]+|total) +[0-9]+( +[0-9]+){9}|fine bodies that miss 'ref on ours': [0-9]+)$" "$f" | sed 's/^/mesh bodies: /' || true
      grep -E "^shell readings: [0-9]+ volume outliers read shell by shell: [0-9]+ explained by inscribed end faces, [0-9]+ explained by faces Revit's export holds twice, [0-9]+ another reason, [0-9]+ unread$" "$f" | sed 's/^/mesh bodies: /' || true
    done
    for f in "$dir"box_bodies_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^(record-box bodies: [0-9]+; fill = Revit's body volume / its bounding box|Ifc[A-Za-z0-9]+ +[0-9]+( +[0-9]+){6} +([0-9]+\.[0-9]+|nan|-)|total +[0-9]+( +[0-9]+){6})$" "$f" | sed 's/^/box bodies: /' || true
    done
    for f in "$dir"saved_meshes_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^((Ifc[A-Za-z0-9]+|total) +[0-9]+( +[0-9]+){5,6}|decided by the fine tessellation \(both files write true curves\): [0-9]+ of [0-9]+ elements)$" "$f" | sed 's/^/saved meshes: /' || true
      grep -E "^(fine tessellation: IfcOpenShell mesher-linear-deflection [0-9.e-]+ m \(1e-5 ft\) and mesher-angular-deflection [0-9.]+ rad, both files; [0-9]+ of [0-9]+ sampled elements meshed|fine (Ifc[A-Za-z0-9]+|total) +[0-9]+( +[0-9]+){7})$" "$f" | sed 's/^/saved meshes: /' || true
    done
    # What the MCP server answered about the model (#1327): the lines starting
    # `mcp answers:` are made of fixed words, numbers and IFC entity names (the
    # scorer's docstring); the lines under them with an element's id, name or
    # value start with spaces and are dropped. The `exit N` line stays, as an
    # `mcp answers:` line, so a crash (no verdict line) is told from a model
    # that was not scored.
    if [ -f "${dir}mcp_answers_vs_ifc.txt" ]; then
      grep -E '^(mcp answers: [A-Za-z0-9 _:.,;()=%/+-]+|exit [0-9]+)$' "${dir}mcp_answers_vs_ifc.txt" \
        | sed -E 's/^exit ([0-9]+)$/mcp answers: exit \1/' || true
    fi
    for f in "$dir"storeys_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E "^([0-9]+ elements, [0-9]+ not on a storey|against Revit's export:| +[0-9]+  Ifc[A-Za-z0-9]+| +[0-9]+  (same storey as Revit's|on no storey; Revit's is (a storey|another container)|on a storey; Revit contains it in <Ifc[A-Za-z0-9]+>|a different storey from Revit's)|  different, by type: Ifc[A-Za-z0-9]+ [0-9]+(, Ifc[A-Za-z0-9]+ [0-9]+)*)$" "$f" | sed 's/^/storeys: /' || true
    done
    # The IFC4 schema's findings on the export (#1430), as counts: the number
    # of findings of the default validation and of the EXPRESS rules check,
    # and for each kind of rule finding its count beside the entity and rule
    # that name it (`IfcShapeRepresentation.CorrectItemsForType`), never the
    # message, which holds the instances. Only the lines of the validation and
    # rules sections are read, up to the property sets. A check skipped for
    # the file's size keeps its line, and a rules check that left no count (it
    # timed out, or did not read the file) says `not read`, so a model it did
    # not cover is told from one it passed.
    for f in "$dir"schema.txt; do
      [ -f "$f" ] || continue
      awk '/^validate: skipped, over [0-9]+ MB$/ { print "schema: " $0; section = ""; next }
           $0 == "validate:" { section = "validate"; next }
           $0 == "rules:" { section = "rules"; rules_seen = 1; next }
           $0 == "arity:" { section = ""; next }
           $0 == "property sets:" { exit }
           section == "validate" && /^findings [0-9]+$/ { print "schema validate: " $0; next }
           section == "rules" && /^findings [0-9]+$/ { print "schema rules: " $0; rules_counted = 1; next }
           section == "rules" && /^[0-9]+ Ifc[A-Za-z0-9]+ Ifc[A-Za-z0-9]+\.[A-Za-z0-9]+:/ {
             split($0, parts, ":"); print "schema rules: " parts[1]; next }
           section != "" && /^unreadable:/ {
             if (section == "rules") rules_counted = 1
             print "schema " section ": not read" }
           END { if (rules_seen && !rules_counted) print "schema rules: not read" }' "$f"
    done
    # How many walls stand at Revit's base and top (tools/re/wall_heights_vs_ifc.py,
    # RE-257, RE-351): counts beside the scorer's fixed group words. The offset
    # between the two files' heights, and the `tag:`, `wall`, `cutter`,
    # `crossed`, `over` and `slab` lines, which carry a Tag or a height, are
    # dropped. Its `exit N` line stays, as a `wall heights:` line.
    for f in "$dir"wall_heights_vs_ifc.txt; do
      [ -f "$f" ] || continue
      sed -nE 's/^walls in both files: ([0-9]+) \(rvt-rs ([0-9]+), Revit ([0-9]+)\); offset -?[0-9.]+ ft$/wall heights: walls in both files \1 (rvt-rs \2, Revit \3)/p' "$f"
      grep -E "^(no wall in both files|cut by the [0-9]+ floors, roofs and footings whose plan box overlaps the wall:|exit [0-9]+)$" "$f" | sed 's/^/wall heights: /' || true
      grep -E "^  (base and top Revit's|base Revit's, top not|top Revit's, base not|neither Revit's|misses in the (same|other) storey|floors, roofs and footings with base (Revit|not Revit)'s and top (Revit|not Revit)'s|(base|top): (cut and Revit's|cut, not Revit's|uncut, Revit cuts|uncut and Revit's)): [0-9]+$" "$f" | sed 's/^  /wall heights: /' || true
    done
    # The wall bodies against Revit's export, per class of wall (#1534): the
    # scorer's summary rows, which are counts and distances.
    for f in "$dir"wall_bodies_vs_ifc.txt; do
      [ -f "$f" ] || continue
      grep -E '^(faces \(across\)|ends \(along\)), (axis-parallel|angled|curved): [0-9]+ walls; within [0-9. ft:]+; max [0-9.]+ ft$' "$f" | sed 's/^/wall bodies: /' || true
    done
    # Which source drew each body (#1534), as counts: a source name in a
    # restricted alphabet and a number, so a moved export can be attributed.
    if [ -f "${dir}diagnostics.json" ]; then
      python3 -I -c 'import json, re, sys
for name, count in sorted(((json.load(open(sys.argv[1])).get("exported") or {}).get("body_sources") or {}).items()):
    if re.fullmatch(r"[a-z0-9_]+", name) and isinstance(count, int):
        print(f"body sources: {name} {count}")' "${dir}diagnostics.json" 2> /dev/null \
        || echo "body sources: not read"
    fi
  } > "$RUNNER_TEMP/counts.txt"
  find "$dir" -mindepth 1 ! -name ifc.sha256 -delete
  mv "$RUNNER_TEMP/counts.txt" "${dir}counts.txt"
  echo "$(basename "$dir"): $(wc -l < "${dir}counts.txt" | tr -d ' ') count lines kept"
done
