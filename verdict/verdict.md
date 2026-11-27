### Verdict

- Exports (`ifc.sha256`): **140 of 145 identical**; **5 differ**: sample-2023-racbasicsampleproject, sample-2024-racbasicsampleproject, sample-2025-racbasicsampleproject, sample-2026-racbasicsampleproject, sample-2027-racbasicsampleproject
- Diagnostics (`diagnostics.json`): **116 of 121 identical**; **5 differ**: sample-2023-racbasicsampleproject, sample-2024-racbasicsampleproject, sample-2025-racbasicsampleproject, sample-2026-racbasicsampleproject, sample-2027-racbasicsampleproject
- Scorer differences: **10 lines in 5 files** beyond what only the head side writes; the first:

```diff
measure-head/sample-2023-racbasicsampleproject/schema.txt: -  placement composition: 122 swept solid(s) carry a Position distinct from their product placement, 0 pinned probe(s) matched
measure-head/sample-2023-racbasicsampleproject/schema.txt: +  placement composition: 121 swept solid(s) carry a Position distinct from their product placement, 0 pinned probe(s) matched
measure-head/sample-2024-racbasicsampleproject/schema.txt: -  placement composition: 138 swept solid(s) carry a Position distinct from their product placement, 0 pinned probe(s) matched
measure-head/sample-2024-racbasicsampleproject/schema.txt: +  placement composition: 137 swept solid(s) carry a Position distinct from their product placement, 0 pinned probe(s) matched
measure-head/sample-2025-racbasicsampleproject/schema.txt: -  placement composition: 123 swept solid(s) carry a Position distinct from their product placement, 0 pinned probe(s) matched
measure-head/sample-2025-racbasicsampleproject/schema.txt: +  placement composition: 122 swept solid(s) carry a Position distinct from their product placement, 0 pinned probe(s) matched
measure-head/sample-2026-racbasicsampleproject/schema.txt: -  placement composition: 123 swept solid(s) carry a Position distinct from their product placement, 0 pinned probe(s) matched
measure-head/sample-2026-racbasicsampleproject/schema.txt: +  placement composition: 122 swept solid(s) carry a Position distinct from their product placement, 0 pinned probe(s) matched
measure-head/sample-2027-racbasicsampleproject/schema.txt: -  placement composition: 123 swept solid(s) carry a Position distinct from their product placement, 0 pinned probe(s) matched
measure-head/sample-2027-racbasicsampleproject/schema.txt: +  placement composition: 122 swept solid(s) carry a Position distinct from their product placement, 0 pinned probe(s) matched
```

