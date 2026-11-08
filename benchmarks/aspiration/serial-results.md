# Aspiration window measurements

Baseline `98d2f0d`. 5 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 9 | 2416.11 | 2401.6–2437.6 | 8,436,304 | 17.4 | 19.2 |
| opening | depth | aspiration | 9 | 2437.03 | 2404.5–2503.4 | 8,435,812 | 17.4 | 19.3 |
| middlegame | depth | baseline | 7 | 1350.53 | 1350.2–1360.0 | 3,223,093 | 4.2 | 6.0 |
| middlegame | depth | aspiration | 7 | 1354.73 | 1347.6–1401.6 | 3,223,082 | 4.2 | 6.1 |
| endgame | depth | baseline | 12 | 4945.86 | 4918.8–5060.1 | 15,400,189 | 76.5 | 78.4 |
| endgame | depth | aspiration | 12 | 4871.35 | 4850.3–5105.1 | 15,090,449 | 75.9 | 77.8 |
| tactics | depth | baseline | 7 | 658.03 | 657.7–659.3 | 2,195,781 | 4.2 | 6.1 |
| tactics | depth | aspiration | 7 | 660.83 | 658.7–661.2 | 2,195,726 | 4.2 | 6.1 |
| pawns | depth | baseline | 12 | 185.26 | 184.9–189.1 | 760,851 | 4.2 | 6.1 |
| pawns | depth | aspiration | 12 | 186.07 | 186.0–187.4 | 760,849 | 4.2 | 6.1 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9, middlegame 7, endgame 12, tactics 7, pawns 12.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Initial aspiration half-width: 64 centipawns; enabled from depth 4.

## Root-window attempts

Counts are medians for the aspiration variant. Narrow attempts include retries; failed windows are retried at the same depth. Ponder counts cover both the warm search and computer search.

| Position | Mode | Narrow attempts | Fail low | Fail high |
|---|---|---:|---:|---:|
| opening | depth | 6 | 0 | 0 |
| middlegame | depth | 4 | 0 | 0 |
| endgame | depth | 9 | 0 | 0 |
| tactics | depth | 4 | 0 | 0 |
| pawns | depth | 9 | 0 | 0 |
