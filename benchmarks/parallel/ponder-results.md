# Parallel load balancing measurements

Baseline `ac1d2b3`. 1 positions, 5 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | ponder | baseline | 8 | 2000.01 | 2000.0–2000.0 | 6,787,413 | 54.0 | 56.4 |
| opening | ponder | balanced | 8 | 2000.06 | 2000.0–2000.1 | 14,253,318 | 65.5 | 68.0 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Pondering is bounded to the time allowance above, followed by a two-second computer search after the recorded human move.

| Position | Variant | Human move | Ponder depth, median | Computer depth, median |
|---|---|---|---:|---:|
| opening | baseline | e2e4 | 9 | 8 |
| opening | balanced | e2e4 | 9 | 8 |

Compiler LTO: disabled.
