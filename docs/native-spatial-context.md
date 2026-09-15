# Native spatial context

The spatial-context projection preserves serialized placement and relationship
fields without inferring containment from geometry.

For instance transforms, the saved transform field is distinct from an
instance origin or placement point. Basis, origin, frame, and length units are
kept as separate values. Root references preserve their native type and field
meaning, including level, phase, host, supercomponent, owner view, unplaced
owner, design option, and room or space cache references. Negative IDs remain
serialized values; missing membership is not converted to a guessed placement.

Linked placement retains the link-instance transform and host link context. A
saved external path refers to another document and does not import its
contents. Linked identities require a separate read of that document.

Shared-coordinate composition, unloaded-link resolution, and universal API
parity remain outside this bounded projection.
