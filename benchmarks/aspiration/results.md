# Aspiration window measurements

Baseline `98d2f0d`. 5 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 9 | 1500.24 | 1424.5–1531.3 | 9,849,274 | 21.1 | 23.4 |
| opening | depth | aspiration | 9 | 1427.49 | 1371.2–1450.5 | 9,638,311 | 21.1 | 23.4 |
| opening | time | baseline | 9 | 3000.01 | 3000.0–3000.0 | 14,932,434 | 33.7 | 36.0 |
| opening | time | aspiration | 9 | 3000.01 | 3000.0–3000.0 | 15,207,592 | 33.7 | 36.0 |
| middlegame | depth | baseline | 7 | 825.24 | 804.4–841.7 | 3,245,038 | 4.2 | 6.5 |
| middlegame | depth | aspiration | 7 | 820.42 | 810.1–846.2 | 3,237,242 | 4.2 | 6.5 |
| middlegame | time | baseline | 8 | 3000.01 | 3000.0–3000.0 | 12,040,809 | 33.7 | 36.0 |
| middlegame | time | aspiration | 8 | 3000.01 | 3000.0–3000.0 | 12,017,425 | 33.7 | 36.0 |
| endgame | depth | baseline | 12 | 2353.19 | 2318.0–2462.1 | 16,924,387 | 65.5 | 67.8 |
| endgame | depth | aspiration | 12 | 2302.71 | 2184.0–2481.1 | 16,137,450 | 65.5 | 67.8 |
| endgame | time | baseline | 12 | 3000.00 | 3000.0–3000.0 | 17,903,194 | 65.5 | 67.9 |
| endgame | time | aspiration | 12 | 3000.01 | 3000.0–3000.0 | 17,722,724 | 65.5 | 67.8 |
| tactics | depth | baseline | 7 | 194.79 | 193.9–194.9 | 2,260,162 | 4.2 | 6.5 |
| tactics | depth | aspiration | 7 | 210.81 | 192.1–221.6 | 2,252,292 | 4.2 | 6.5 |
| tactics | time | baseline | 8 | 3000.07 | 3000.1–3000.1 | 22,214,530 | 49.6 | 52.0 |
| tactics | time | aspiration | 8 | 3000.06 | 3000.1–3000.1 | 20,853,122 | 47.2 | 49.6 |
| pawns | depth | baseline | 12 | 116.60 | 115.3–120.4 | 826,972 | 4.2 | 6.5 |
| pawns | depth | aspiration | 12 | 117.76 | 114.8–124.5 | 790,514 | 4.2 | 6.5 |
| pawns | time | baseline | 16 | 3000.00 | 3000.0–3000.0 | 15,291,676 | 65.5 | 67.9 |
| pawns | time | aspiration | 16 | 3000.00 | 3000.0–3000.0 | 15,378,323 | 65.5 | 67.9 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9, middlegame 7, endgame 12, tactics 7, pawns 12.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Initial aspiration half-width: 64 centipawns; enabled from depth 4.

## Root-window attempts

Counts are medians for the aspiration variant. Narrow attempts include retries; failed windows are retried at the same depth. Ponder counts cover both the warm search and computer search.

| Position | Mode | Narrow attempts | Fail low | Fail high |
|---|---|---:|---:|---:|
| opening | depth | 6 | 0 | 0 |
| opening | time | 7 | 0 | 0 |
| middlegame | depth | 4 | 0 | 0 |
| middlegame | time | 6 | 0 | 0 |
| endgame | depth | 9 | 0 | 0 |
| endgame | time | 10 | 0 | 0 |
| tactics | depth | 4 | 0 | 0 |
| tactics | time | 6 | 0 | 0 |
| pawns | depth | 9 | 0 | 0 |
| pawns | time | 14 | 0 | 0 |
