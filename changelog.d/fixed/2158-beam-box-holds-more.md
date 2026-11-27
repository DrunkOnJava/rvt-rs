- **A beam whose record box holds more than the beam is drawn from its saved
  mesh** (#2158). A beam drawn along its location line takes its section from
  its record box; where that section is more than five times as wide or as
  deep as the beam's own saved mesh across the line, and the mesh decodes
  whole, the beam is drawn from the mesh instead, and its base quantities come
  from it. On a Revit 2024 project reported on #2158, nine beams were drawn
  8.3 m deep, the box's height, as wall-sized sheets. On the 101 models Measure
  reads no beam is over 4.43 times its mesh's depth, so none of them changes.
