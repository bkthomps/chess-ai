# Aspiration window measurements

Baseline `98d2f0d`. 5 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 9 | 2410.16 | 2405.2–2415.0 | 8,436,304 | 17.4 | 19.3 |
| opening | depth | aspiration | 9 | 2043.55 | 2037.1–2077.4 | 7,120,469 | 16.8 | 18.7 |
| middlegame | depth | baseline | 7 | 1341.84 | 1338.7–1345.2 | 3,223,093 | 4.2 | 6.1 |
| middlegame | depth | aspiration | 7 | 1348.25 | 1343.9–1360.6 | 3,219,062 | 4.2 | 6.0 |
| endgame | depth | baseline | 12 | 4904.12 | 4903.1–4929.1 | 15,400,189 | 76.5 | 78.4 |
| endgame | depth | aspiration | 12 | 4823.50 | 4821.1–4883.3 | 15,168,185 | 75.8 | 77.8 |
| tactics | depth | baseline | 7 | 658.27 | 657.5–663.8 | 2,195,781 | 4.2 | 6.0 |
| tactics | depth | aspiration | 7 | 828.98 | 828.6–832.4 | 2,511,051 | 4.2 | 6.0 |
| pawns | depth | baseline | 12 | 187.22 | 181.4–187.6 | 760,851 | 4.2 | 6.1 |
| pawns | depth | aspiration | 12 | 183.27 | 182.2–192.2 | 764,252 | 4.2 | 6.1 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9, middlegame 7, endgame 12, tactics 7, pawns 12.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Initial aspiration half-width: 32 centipawns; enabled from depth 4.

## Root-window attempts

Counts are medians for the aspiration variant. Narrow attempts include retries; failed windows are retried at the same depth. Ponder counts cover both the warm search and computer search.

| Position | Mode | Narrow attempts | Fail low | Fail high |
|---|---|---:|---:|---:|
| opening | depth | 7 | 1 | 0 |
| middlegame | depth | 4 | 0 | 0 |
| endgame | depth | 10 | 1 | 0 |
| tactics | depth | 8 | 2 | 2 |
| pawns | depth | 10 | 1 | 0 |
