- **glTF keeps the transparency of an element drawn whole (#1812).** An
  element whose body the exporter draws as one piece (a glass door or light
  fixture whose type draws in several materials) keeps its category colour but
  now takes the opacity of its most transparent material, instead of the
  category's opaque alpha; a single material or profile whose colour was not
  read contributes its alpha too. All-opaque or unread materials stay opaque,
  and the material still marks the alpha `"rvt-rs"` in its `extras`.
