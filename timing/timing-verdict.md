### Timing verdict

| group | files | time head / base | peak head / base |
|---|---:|---|---|
| Models | 16 | median 0.998 (0.980 to 1.023) | median 1.000 (0.969 to 1.001) |
| Samples of Revit 2023 to 2027 | 29 | median 1.012 (0.990 to 1.023) | median 1.000 (0.984 to 1.026) |
| Samples of Revit 2015 to 2022 | 30 | median 1.003 (0.871 to 1.053) | median 1.000 (0.988 to 1.006) |

- The three files with the longest base time: 2024-snowdon_towers_sample_architectural time 1.017, peak 1.000; ARK_NordicLCA_Office_Timber_BuildingPermit time 1.001, peak 1.001; ARK_NordicLCA_Office_Concrete_As-Built time 0.998, peak 1.000
- The three files with the highest base peak: ARK_NordicLCA_Office_Timber_BuildingPermit time 1.001, peak 1.001; ARK_NordicLCA_Office_Concrete_BuildingPermit time 1.014, peak 1.000; 2024-snowdon_towers_sample_architectural time 1.017, peak 1.000
- More than 3% slower: **2017-rac_advanced_sample_project** 1.053; **2019-rme_advanced_sample_project** 1.035
- Peak more than 2% higher: none
- Peak in two values from round to round, rounds more than 2% apart, so the median is either (higher above only where every head round is): **2023-racadvancedsampleproject** 0.997 (base 301 309 307, head 306 311 299 MiB); **2024-snowdon_towers_sample_hvac** 0.985 (base 276 262 262, head 257 258 262 MiB); **2024-snowdon_towers_sample_plumbing** 1.026 (base 497 508 521, head 521 521 521 MiB); **2025-racadvancedsampleproject** 0.984 (base 309 302 309, head 304 303 308 MiB); **2027-racadvancedsampleproject** 0.990 (base 308 310 312, head 303 307 313 MiB)

