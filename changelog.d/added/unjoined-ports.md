- **On Revit 2025, every family-instance port is placed, joined or not
  (#869).** A port no duct or pipe joins now stands at its connector: the
  point and axis its connector entry gives in the symbol that draws the
  instance (RE-215), through the instance's transform (RE-87). On Revit 2025
  all 53 such ports of RE1 Electrical, 29 of Mechanical and 18 of Plumbing
  are within 0.01 ft of Revit's, each with Revit's axis; they had no
  placement. The type's symbol is not enough: fittings sized per instance
  have symbols of their own, and it put 11 of those ports up to 0.81 ft off.
  Ports joined to a duct or pipe stay at their partner's point (#922).
