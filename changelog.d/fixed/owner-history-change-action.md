- **The owner history meets IFC4's `CorrectChangeAction` rule.** Every
  export wrote `IfcOwnerHistory` with `ChangeAction` `.ADDED.` and no
  `LastModifiedDate`, which IFC4's WHERE rule forbids (IfcOpenShell's
  EXPRESS rule check found it on every reference export). It is now
  `.NOCHANGE.` with no last-modifying user or application, as Revit's own
  export writes it.
