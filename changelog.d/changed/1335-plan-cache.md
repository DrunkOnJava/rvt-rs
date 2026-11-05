- **A pass over a file's records makes the plan of each class once, not once
  for each record (#1335).** Decoding a record works out, for each class it
  meets, the chain of classes and the key of each field. A pass over a Revit
  2015 to 2023 project's records did that again for every record, which was
  3.1% of the time of the 33 samples' exports. The passes now keep the plans
  between records (`native_parameters::PlanCache`). The graphs are the same. No
  export changes. The figures are in the pull request.
