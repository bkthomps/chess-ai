# Parallel load balancing measurements

Baseline `ac1d2b3`. 5 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 9 | 1439.21 | 1433.3–1518.4 | 9,712,101 | 21.1 | 23.3 |
| opening | depth | balanced | 9 | 1075.80 | 1051.8–1324.1 | 10,064,744 | 24.0 | 26.3 |
| middlegame | depth | baseline | 7 | 898.00 | 852.2–912.2 | 3,248,065 | 4.2 | 6.4 |
| middlegame | depth | balanced | 7 | 572.23 | 558.2–593.4 | 3,245,771 | 4.2 | 6.5 |
| endgame | depth | baseline | 12 | 2583.81 | 2527.6–2772.6 | 16,098,197 | 65.5 | 67.8 |
| endgame | depth | balanced | 12 | 2303.86 | 2202.9–2384.6 | 15,863,767 | 65.5 | 67.8 |
| tactics | depth | baseline | 7 | 266.06 | 260.8–334.1 | 2,268,198 | 4.2 | 6.4 |
| tactics | depth | balanced | 7 | 263.84 | 240.4–274.3 | 2,275,706 | 4.2 | 6.5 |
| pawns | depth | baseline | 12 | 142.57 | 139.1–158.3 | 835,975 | 4.2 | 6.4 |
| pawns | depth | balanced | 12 | 86.40 | 84.6–88.2 | 829,527 | 4.2 | 6.5 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9, middlegame 7, endgame 12, tactics 7, pawns 12.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Compiler LTO: fat.
