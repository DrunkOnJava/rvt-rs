- **Curtain mullions' `NetVolume` (#1016, RE-243).** A mullion whose type
  has its own rectangular or square section, standing on a straight curtain
  wall whose profile is not edited, carries `Qto_MemberBaseQuantities.NetVolume`:
  its section times its length, the volume of the extrusion Revit's export
  draws it as. On the BIM4LCA office that is 7,795 of 10,003 mullions as
  built and 7,823 of 10,031 at building permit, each Revit's value. Mullions
  on curved curtain walls or ones with an edited profile, some of which
  Revit draws as cut meshes, get none.
