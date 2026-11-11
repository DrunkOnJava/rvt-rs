- **A railing's type is named by the latest copy of its type object, which gives the housing's railings
  their `Reference` and `Height` (#1193, RE-431).** Every railing type of the BIM4LCA housing holds an
  older copy with a Finnish name and a newer one with the English name Revit's export carries, so the
  names never agreed and no type was attached. The name from the latest partition that holds one is
  Revit's on all 198 of the housing's railings, and the 237 railings of the housing, the timber office
  at permit and the two terrain office files gain their type, `Pset_RailingCommon.Reference` and
  `Height` and `Pset_EnvironmentalImpactIndicators.Reference`, each Revit's (4 to 241 of 241 and 237
  more, none different); 7 of 126 exports move and the other 119 are byte-identical. A railing type
  object is written for each of them, and `Qto_RailingBaseQuantities.Length` stays out of the housing.
