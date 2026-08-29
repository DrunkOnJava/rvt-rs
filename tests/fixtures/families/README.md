# Redistributable family fixtures

These `.rfa` files are MIT-licensed samples from upstream repos (not the
Autodesk `rac_basic_sample_family` corpus). Each binary has a sibling
`<name>.license.json` with source URL, SPDX license, SHA256, and size.

| Fixture | Source | License | Role |
|---|---|---|---|
| `empty.rfa` | `DynamoDS/RevitTestFramework` | MIT | Always-on stream-patch corpus (grow / shrink / multi / missing) |

Project-shaped fixtures are generated at test time by `gen-fixture`
(license-free synthetic CFB) rather than committed here.
