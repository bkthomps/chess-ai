# Principal Variation Search measurements

Baseline `6765813`. 1 positions, 7 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| tactics | depth | baseline | 7 | 950.46 | 693.1–997.5 | 2,230,440 | 4.2 | 6.1 |
| tactics | depth | pvs | 7 | 676.25 | 672.5–995.9 | 2,195,781 | 4.2 | 6.0 |

Time-limited searches use 3 seconds. Fixed-depth targets: tactics 7.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.
