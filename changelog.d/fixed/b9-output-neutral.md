- **The viewer's schedule names a layered element's material, the diagnostics
  count what `--revit-property-sets` adds, and a few messages say what the
  export does.** Found by the batch 9 export audit; no IFC value changes.
  - The viewer's schedule read only an element's single material, so a wall
    of layers showed none where the elements CSV names its layer set. Both
    now read one rule: profile set, then layer set, then single material,
    then constituents.
  - `rvt-ifc --revit-property-sets` added its sets after the export's counts
    were taken, so `exported.total_entities` missed them; the counts are taken
    again after it, as after every other step that changes the model.
  - With no Revit Level recovered, the warning said the IFC used a fallback
    storey. It has none, and its elements are contained in the building
    (RE-124); the warning says so.
  - The element panel printed a length that rounds to zero as `-0.000 ft`, as
    its placement rows never do.
  - A glTF export drops a material index past the model's materials, which
    would have made the file invalid, as the IFC writer already drops it.
