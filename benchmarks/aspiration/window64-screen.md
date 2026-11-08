# Aspiration window measurements

Baseline `98d2f0d`. 5 positions, 1 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 7 | 98.37 | 98.4–98.4 | 380,559 | 4.2 | 6.0 |
| opening | depth | aspiration | 7 | 143.11 | 143.1–143.1 | 380,558 | 4.2 | 6.0 |
| middlegame | depth | baseline | 5 | 186.16 | 186.2–186.2 | 387,106 | 4.2 | 6.0 |
| middlegame | depth | aspiration | 5 | 184.33 | 184.3–184.3 | 383,899 | 4.2 | 6.1 |
| endgame | depth | baseline | 10 | 524.50 | 524.5–524.5 | 1,706,412 | 8.4 | 10.3 |
| endgame | depth | aspiration | 10 | 520.65 | 520.6–520.6 | 1,671,187 | 8.4 | 10.3 |
| tactics | depth | baseline | 5 | 36.10 | 36.1–36.1 | 120,488 | 4.2 | 6.1 |
| tactics | depth | aspiration | 5 | 36.13 | 36.1–36.1 | 120,468 | 4.2 | 6.0 |
| pawns | depth | baseline | 10 | 41.48 | 41.5–41.5 | 158,056 | 4.2 | 6.0 |
| pawns | depth | aspiration | 10 | 39.35 | 39.4–39.4 | 158,056 | 4.2 | 6.0 |

Time-limited searches use 1 seconds. Fixed-depth targets: opening 7, middlegame 5, endgame 10, tactics 5, pawns 10.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Initial aspiration half-width: 64 centipawns; enabled from depth 4.

## Root-window attempts

Counts are medians for the aspiration variant. Narrow attempts include retries; failed windows are retried at the same depth. Ponder counts cover both the warm search and computer search.

| Position | Mode | Narrow attempts | Fail low | Fail high |
|---|---|---:|---:|---:|
| opening | depth | 4 | 0 | 0 |
| middlegame | depth | 2 | 0 | 0 |
| endgame | depth | 7 | 0 | 0 |
| tactics | depth | 2 | 0 | 0 |
| pawns | depth | 7 | 0 | 0 |
