- **A floor of the Structural Foundations category is an `IfcSlab` with
  `BASESLAB`, and a horizontal bracing an `IfcMember` (#1841, RE-457).**
  `StructuralFoundation` was mapped to `IfcFooting` and `StructuralFraming`
  to `IfcBeam` whole. On a reporter's Revit 2027 structural model, Revit's
  export writes the 59 foundation slabs as `IfcSlab` (`BASESLAB` on their
  types) and the one framing instance with stored structural usage 8
  (Horizontal Bracing) as `IfcMember`, where the 914 with usage 3 and 6 are
  `IfcBeam`. A floor of the category, which its record names as the class
  `Floor`, is now written as the slab, and a framing instance of usage 8 as
  the member; family-instance and wall footings and every other usage are as
  before. No model Measure holds with a Revit export has a foundation or a
  bracing, so the change is checked on the reporter's counts after the
  release; the member's `PredefinedType` is not read yet.
