# Native saved spatial boundaries

The bounded boundary projection follows saved ownership rather than proximity:

```text
RoomElem -> m_cachedCircuitId
LevelRoomPlan -> PlanTopology
PlanTopology -> explicit room list and saved circuits
Circuit -> directed SegmentKey values
```

The join is scoped by owner identity, level, explicit room membership, and the
local cached circuit ID. A circuit ID alone is not globally unique. Segment
keys preserve host-or-link instance ID, linked element ID, major index, and
signed sub-index. Native negative signed 32-bit sub-indices use bitwise
complement (`-1 -> 0`, `-2 -> 1`) and reverse the directed point order.

The projection distinguishes saved carrier topology from evaluated finish,
center, or core boundaries. Carrier coordinates are retained as 2D native
document coordinates in feet. A closed saved loop is not promoted to a 3D
space boundary, physical containment relation, or canonical world-model edge.

The implementation refuses or reports diagnostics for absent or ambiguous
topology membership, missing cached circuits, invalid reference sentinels,
disconnected loops, unsupported linked/slanted/curved/variable wall cases, and
unqualified boundary evaluation. Native-host references may use
`linked_element_id = -1`; this is distinct from an invalid zero or less-than
`-1` sentinel.

The implementation lives in
[`native_spatial_boundaries.rs`](../src/native_spatial_boundaries.rs) and its
evaluation module. The projection status
`resolved_saved_carrier_topology_not_evaluated_boundary` is intentionally
stronger than “record decoded” and weaker than evaluated boundary parity.
