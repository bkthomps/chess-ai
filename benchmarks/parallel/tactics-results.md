# Parallel load balancing measurements

Baseline `ac1d2b3`. 1 positions, 15 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| tactics | depth | baseline | 7 | 196.74 | 187.3–238.3 | 2,250,909 | 4.2 | 6.5 |
| tactics | depth | balanced | 7 | 186.95 | 177.3–227.2 | 2,256,683 | 4.2 | 6.5 |

Time-limited searches use 3 seconds. Fixed-depth targets: tactics 7.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Compiler LTO: disabled.
