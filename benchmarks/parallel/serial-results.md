# Parallel load balancing measurements

Baseline `ac1d2b3`. 5 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 9 | 2411.13 | 2410.5–2437.0 | 8,436,304 | 17.4 | 19.3 |
| opening | depth | balanced | 9 | 2424.80 | 2407.6–2482.6 | 8,436,304 | 17.4 | 19.3 |
| middlegame | depth | baseline | 7 | 1338.19 | 1338.2–1340.7 | 3,223,093 | 4.2 | 6.1 |
| middlegame | depth | balanced | 7 | 1341.89 | 1340.2–1342.0 | 3,223,093 | 4.2 | 6.1 |
| endgame | depth | baseline | 12 | 4911.56 | 4896.1–4981.8 | 15,400,189 | 76.5 | 78.4 |
| endgame | depth | balanced | 12 | 4998.30 | 4901.9–5005.9 | 15,400,189 | 76.5 | 78.4 |
| tactics | depth | baseline | 7 | 666.52 | 665.3–669.2 | 2,195,781 | 4.2 | 6.1 |
| tactics | depth | balanced | 7 | 666.30 | 664.0–667.9 | 2,195,781 | 4.2 | 6.1 |
| pawns | depth | baseline | 12 | 189.37 | 185.8–189.4 | 760,851 | 4.2 | 6.1 |
| pawns | depth | balanced | 12 | 190.16 | 188.1–192.3 | 760,851 | 4.2 | 6.1 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9, middlegame 7, endgame 12, tactics 7, pawns 12.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Compiler LTO: disabled.
