# Native spatial context

`native_spatial_context` preserves serialized placement and relationship
observations without inferring containment from geometry.

For instance transforms, `m_pInstanceInfo -> InstanceInfo.m_Trf` is the cached
transform. The serialized matrix orientation and translation are retained as
source data; an instance placement point such as `m_instOrigin` is a separate
field and must not replace the cached transform. Basis, origin, frame, and
length-unit qualifications remain explicit.

Root references preserve their native type and field names, including level,
phase, host, supercomponent, owner view, unplaced owner, design option, and
room/space cache IDs. Negative IDs remain serialized observations. Group and
unplaced membership is only reported when the owning native chain is present;
missing membership is not converted to a guessed placement.

Linked placement retains the link-instance transform and the host link context.
A saved external path is a reference to another document, not its imported
contents or a globally trusted namespace. Linked element identities require a
separate extraction of the referenced document. Shared-coordinate composition,
unloaded-link resolution, and universal API parity remain outside this bounded
projection.

The implementation is in
[`native_spatial_context.rs`](../src/native_spatial_context.rs). It retains
record provenance for each relationship and keeps source observations separate
from later reconciliation or canonical containment decisions.
