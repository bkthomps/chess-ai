# Aspiration window measurements

Baseline `98d2f0d`. 1 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | ponder | baseline | 8 | 2000.01 | 2000.0–2000.0 | 6,761,575 | 54.3 | 56.7 |
| opening | ponder | aspiration | 8 | 2000.01 | 2000.0–2000.0 | 6,325,182 | 54.8 | 57.2 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Pondering is bounded to the time allowance above, followed by a two-second computer search after the recorded human move.

| Position | Variant | Human move | Ponder depth, median | Computer depth, median |
|---|---|---|---:|---:|
| opening | baseline | e2e4 | 9 | 8 |
| opening | aspiration | e2e4 | 9 | 8 |

Initial aspiration half-width: 32 centipawns; enabled from depth 4.

## Root-window attempts

Counts are medians for the aspiration variant. Narrow attempts include retries; failed windows are retried at the same depth. Ponder counts cover both the warm search and computer search.

| Position | Mode | Narrow attempts | Fail low | Fail high |
|---|---|---:|---:|---:|
| opening | ponder | 16 | 3 | 0 |
