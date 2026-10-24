- **A door's or window's opening is the outline its symbol cuts (RE-215,
  #227).** On Revit 2024 and 2025, where its type gives no opening, a door
  or window cuts its host along the outline the symbol that draws it cuts,
  placed by the instance's transform (RE-87) and through the wall. 124 of
  Core Interior's 132 doors, its 6 windows and RE1 Architecture's 4 doors
  now cut Revit's opening within 0.001 ft; they were up to 0.26 ft off.
  Core Interior's other 8 doors have no transform read and keep their box.
