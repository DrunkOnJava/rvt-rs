- **A crafted file can no longer exhaust the stack or hang the export
  through the chains and counts it sets (#1845).** A wall's profile cuts,
  one boolean level each, are written and meshed by a loop instead of by
  recursion. The scene graph hangs an element at most 16 hosts below its
  storey, as the IFC's storey lookup already did, so doors chained through
  each other no longer recurse as deep as the chain. The parts a covering
  element cuts from a wall's plan (1,024), the pieces left of a stub wall
  inside its neighbours (128), a roof envelope's points (about a million),
  the mullion ends in one grid cell (64) and the outline tests of space
  containment (a billion) each have a bound no real model comes near. Past
  one, that one step is left undone: the wall keeps its plan, a crowded
  cell counts no partner, nothing is contained in a space. No real file of
  the measured corpus changes.
