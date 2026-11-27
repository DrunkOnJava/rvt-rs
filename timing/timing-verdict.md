### Timing verdict

| group | files | time head / base | peak head / base |
|---|---:|---|---|
| Models | 16 | median 0.998 (0.981 to 1.016) | median 1.000 (0.994 to 1.000) |
| Samples of Revit 2023 to 2027 | 29 | median 1.003 (0.986 to 1.014) | median 1.000 (0.946 to 1.010) |
| Samples of Revit 2015 to 2022 | 30 | median 1.001 (0.962 to 1.031) | median 1.000 (0.988 to 1.000) |

- The three files with the longest base time: ARK_NordicLCA_Office_Timber_BuildingPermit time 1.016, peak 1.000; 2024-snowdon_towers_sample_architectural time 1.011, peak 1.000; ARK_NordicLCA_Office_Concrete_As-Built time 0.992, peak 1.000
- The three files with the highest base peak: ARK_NordicLCA_Office_Timber_BuildingPermit time 1.016, peak 1.000; ARK_NordicLCA_Office_Concrete_BuildingPermit time 0.994, peak 1.000; 2024-snowdon_towers_sample_architectural time 1.011, peak 1.000
- More than 3% slower: **2022-racadvancedsampleproject** 1.031
- Peak more than 2% higher: none
- Peak in two values from round to round, rounds more than 2% apart, so the median is either (higher above only where every head round is): **2023-racadvancedsampleproject** 0.987 (base 308 307 312, head 309 304 298 MiB); **2024-rmebasicsampleproject** 1.000 (base 415 415 415, head 401 415 415 MiB); **2024-snowdon_towers_sample_hvac** 0.946 (base 277 262 277, head 262 276 262 MiB); **2024-snowdon_towers_sample_plumbing** 0.985 (base 521 521 494, head 513 494 520 MiB)

