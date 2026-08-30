# -*- coding: utf-8 -*-
# pyright: reportMissingImports=false, reportAttributeAccessIssue=false, reportUndefinedVariable=false
"""pyRevit push button: generate the ES-remap-00 fixture family + observations.

Run this in a NEW, EMPTY project document created from Revit's default
project template (the seed needs one Level and nothing else). Output goes to
tools/oracle/out/<revit-version>-<timestamp>/ next to this extension, or to
the folder given in RVT_ORACLE_OUT.

UNTESTED outside Revit — see tools/oracle/runner/pyrevit/README.md.
"""

__title__ = "Run ES-remap-00"
__doc__ = "Build the S_All seed, then run N1-N4, R1/R2, C1/C2, C3a/C4a and write observations."

import os
import time

from pyrevit import forms  # noqa: F401  (pyRevit ships with the host)

import rvt_oracle

uiapp = __revit__  # noqa: F821  (injected by pyRevit)
doc = uiapp.ActiveUIDocument.Document

if doc.IsFamilyDocument:
    forms.alert("Open a new empty PROJECT (default template), not a family.", exitscript=True)

out_root = os.environ.get("RVT_ORACLE_OUT")
if not out_root:
    here = os.path.dirname(os.path.abspath(__file__))
    out_root = os.path.normpath(os.path.join(here, "..", "..", "..", "..", "..", "out"))
stamp = "{}-{}".format(uiapp.Application.VersionNumber, time.strftime("%Y%m%d-%H%M%S"))
out_dir = os.path.join(out_root, stamp)

only = os.environ.get("RVT_ORACLE_ONLY")  # e.g. "N1,R1,C1" to run a subset
only = [s.strip() for s in only.split(",")] if only else None

runner = rvt_oracle.Runner(uiapp, out_dir)
runner.run_all(doc, only=only)
forms.alert("ES-remap-00 run finished.\n\n{} observations\n{}".format(len(runner.observations), out_dir))
