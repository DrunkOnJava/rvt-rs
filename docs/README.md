# Documentation

The pages under `docs/` are of three kinds. The user pages say what rvt-rs does now and are kept current with the code. CI checks that they cite only reports that exist and carry none of the project's internal planning ids (`tools/ci/api-doc-ids.py`). The contributor pages describe how the project is run. The research record is everything written as work happened, kept as it was written.

## For users

- [User guide](user-guide.md): inspect, export and schedule Revit files from the command line.
- [Install](install.md): the binaries, `cargo install`, `pip install` and the container image.
- [Python](python.md): the `rvt` package's API, with examples that run in CI.
- [Status](status.md) and [compatibility](compatibility.md): what each Revit release, file type and element class gives, with the evidence. The [support matrix](support-matrix.json) is the machine-readable ceiling.
- [Supported profile](supported-profile.md): the inputs rvt-rs is measured on.
- [Export diagnostics](export-diagnostics.md) and [diagnostic semantics](diagnostic-semantics.md): what an export says it approximated or left out.
- [`rvt-inspect`](inspect.md) and [`rvt-ifc-compare`](ifc-compare.md): checking a file before relying on it, and comparing an export with Revit's.
- [Validation evidence](validation-evidence.md) and the [verification protocol](verification-protocol.md): how an IFC export is checked, and what "verified" means.
- [Demos](demos.md) and [benchmarks](benchmarks.md): reproducible outputs and timings.
- [Viewer API](viewer-api.md) and [viewer privacy posture](viewer-privacy-posture.md): the browser viewer's bindings, and why it uploads nothing.
- [JSON schemas](schemas/README.md): what the CLIs print.
- [1.0 release criteria](release-1.0-criteria.md): what 1.0 requires, and where each requirement stands.

## For contributors and maintainers

[Maintaining](maintaining.md), the [release checklist](release-checklist.md), the [contribution map](contribution-map.md), the [corpus guide](corpus.md), [corpus intake](corpus-intake.md) and [sources](corpus-sources.md), the [reference models](reference-models.md), [extending the element decoders](extending-layer-5b.md), the [viewer build pipeline](viewer-build-pipeline.md), the [supply-chain policy](supply-chain-policy.md), and the decision records under [`decisions/`](decisions/).

## Research record

The [reconnaissance report](rvt-moat-break-reconnaissance.md), the dated probe notes and session logs (the pages whose names carry a date), [`research/`](research/), [`recon/`](recon/), [`re/`](re/), the OctetProof specification drafts, the coordination notes, [`blog/`](blog/) and [`launch/`](launch/). These pages record what was known when they were written, and they use the project's internal ids (backlog items, work-plan lanes and phases) as they were used then. The format findings themselves are the RE reports under [`reports/element-framing/`](../reports/element-framing/).
