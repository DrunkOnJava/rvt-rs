# Native saved spatial boundaries

The boundary projection follows saved ownership and explicit relationships:

```text
RoomElem -> cached circuit
LevelRoomPlan -> plan topology
PlanTopology -> saved room list and circuits
Circuit -> directed segment values
```

The join is scoped by owner identity, level, explicit room membership, and the
local cached circuit. Segment keys preserve host or link instance, linked
element, major index, and signed sub-index. Native negative sub-indices use the
format's signed encoding and preserve directed point order.

Saved carrier topology is distinct from evaluated finish, center, or core
boundaries. Carrier coordinates remain two-dimensional document coordinates in
feet. A closed saved loop is not promoted to a three-dimensional space
boundary, physical containment relation, or canonical world-model edge.

Absent or ambiguous membership, missing circuits, invalid sentinels,
disconnected loops, linked or slanted cases, curved or variable walls, and
unqualified evaluation produce diagnostics or explicit refusal states.
