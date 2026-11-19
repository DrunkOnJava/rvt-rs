- **A stair run is called spiral only where its treads lie about one centre
  (#2046).** rvt-rs wrote `SPIRAL` for a flight, and `SPIRAL_STAIR` for its
  stair, wherever every tread of the run gives `m_runType` 2. A reporter's
  Revit 2027 model with no spiral run has three such runs, which Revit's export
  does not call spiral (their flights' types are `USERDEFINED` with an
  `ElementType` of `NOTDEFINED`), as revit-ifc gives `NOTDEFINED` for any run
  whose style is not Straight, Spiral or Winder. Such a run is now a spiral only where its treads
  pass the spiral test the 2024 spiral treads are drawn with (one centre, one
  pair of radii, each tread beside and above the one before), and its flight and
  stair keep `NOTDEFINED` otherwise.
