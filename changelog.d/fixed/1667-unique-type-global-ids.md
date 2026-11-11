- **Every type object has a GlobalId of its own (#1667).** A type read for
  several symbols can leave several type objects of one entity and Tag that all
  take the writer's fallback GlobalId. The BIM4LCA office at permit (timber)
  wrote one mullion type as 6 `IfcMemberType` with one GlobalId, and their
  property sets and `IfcRelDefinesByType` repeated it, 24 `IfcRoot.UR1` findings
  in IfcOpenShell's validation. The first of them keeps the fallback and each
  later one takes a GlobalId that carries its place in the list.
