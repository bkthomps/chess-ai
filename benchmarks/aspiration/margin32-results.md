# Aspiration window measurements

Baseline `98d2f0d`. 5 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 9 | 1456.72 | 1444.0–1489.8 | 9,950,932 | 21.3 | 23.6 |
| opening | depth | aspiration | 9 | 1353.51 | 1322.7–1594.2 | 9,097,788 | 21.0 | 23.3 |
| opening | time | baseline | 9 | 3000.01 | 3000.0–3000.0 | 14,641,940 | 33.7 | 36.0 |
| opening | time | aspiration | 9 | 3000.01 | 3000.0–3000.0 | 13,780,163 | 33.7 | 35.8 |
| middlegame | depth | baseline | 7 | 837.23 | 813.8–946.4 | 3,247,140 | 4.2 | 6.5 |
| middlegame | depth | aspiration | 7 | 810.09 | 808.2–814.7 | 3,229,465 | 4.2 | 6.5 |
| middlegame | time | baseline | 8 | 3000.02 | 3000.0–3000.1 | 11,811,085 | 33.7 | 36.0 |
| middlegame | time | aspiration | 8 | 3000.01 | 3000.0–3000.0 | 11,914,089 | 33.7 | 36.0 |
| endgame | depth | baseline | 12 | 2107.92 | 2090.3–2195.7 | 15,941,613 | 65.5 | 67.8 |
| endgame | depth | aspiration | 12 | 2198.88 | 2114.2–2314.6 | 16,376,105 | 65.5 | 67.9 |
| endgame | time | baseline | 12 | 3000.01 | 3000.0–3000.0 | 18,803,154 | 65.5 | 67.9 |
| endgame | time | aspiration | 12 | 3000.00 | 3000.0–3000.0 | 18,367,782 | 65.5 | 67.9 |
| tactics | depth | baseline | 7 | 200.90 | 200.7–355.7 | 2,274,466 | 4.2 | 6.5 |
| tactics | depth | aspiration | 7 | 263.84 | 251.0–416.9 | 2,581,645 | 4.2 | 6.6 |
| tactics | time | baseline | 8 | 3000.08 | 3000.0–3000.1 | 20,821,270 | 47.8 | 50.2 |
| tactics | time | aspiration | 8 | 3000.01 | 3000.0–3000.0 | 16,020,167 | 50.4 | 52.8 |
| pawns | depth | baseline | 12 | 115.23 | 111.0–119.8 | 796,632 | 4.2 | 6.5 |
| pawns | depth | aspiration | 12 | 121.62 | 118.9–125.4 | 822,721 | 4.2 | 6.5 |
| pawns | time | baseline | 16 | 3000.00 | 3000.0–3000.0 | 14,764,853 | 65.5 | 67.9 |
| pawns | time | aspiration | 16 | 3000.01 | 3000.0–3000.0 | 14,487,253 | 65.5 | 67.9 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9, middlegame 7, endgame 12, tactics 7, pawns 12.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Initial aspiration half-width: 32 centipawns; enabled from depth 4.

## Root-window attempts

Counts are medians for the aspiration variant. Narrow attempts include retries; failed windows are retried at the same depth. Ponder counts cover both the warm search and computer search.

| Position | Mode | Narrow attempts | Fail low | Fail high |
|---|---|---:|---:|---:|
| opening | depth | 7 | 1 | 0 |
| opening | time | 8 | 1 | 0 |
| middlegame | depth | 4 | 0 | 0 |
| middlegame | time | 6 | 0 | 0 |
| endgame | depth | 10 | 1 | 0 |
| endgame | time | 11 | 1 | 0 |
| tactics | depth | 8 | 2 | 2 |
| tactics | time | 12 | 3 | 3 |
| pawns | depth | 10 | 1 | 0 |
| pawns | time | 15 | 1 | 0 |
