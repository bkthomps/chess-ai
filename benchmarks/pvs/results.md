# Principal Variation Search measurements

Baseline `6765813`. 5 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 9 | 1655.78 | 1588.5–1658.5 | 9,894,140 | 22.5 | 24.8 |
| opening | depth | pvs | 9 | 1566.26 | 1521.7–1839.1 | 10,231,691 | 22.0 | 24.4 |
| opening | time | baseline | 9 | 3000.01 | 3000.0–3000.0 | 13,857,792 | 33.7 | 35.9 |
| opening | time | pvs | 9 | 3000.00 | 3000.0–3000.0 | 14,016,042 | 33.7 | 35.9 |
| middlegame | depth | baseline | 7 | 1475.58 | 1439.7–1793.5 | 4,231,078 | 4.2 | 6.5 |
| middlegame | depth | pvs | 7 | 842.39 | 837.1–843.9 | 3,244,550 | 4.2 | 6.5 |
| middlegame | time | baseline | 7 | 3000.07 | 3000.1–3000.1 | 10,383,403 | 25.2 | 27.6 |
| middlegame | time | pvs | 8 | 3000.03 | 3000.0–3000.1 | 11,815,858 | 33.7 | 36.0 |
| endgame | depth | baseline | 12 | 2402.95 | 2389.1–2412.3 | 17,171,981 | 65.5 | 67.9 |
| endgame | depth | pvs | 12 | 2695.08 | 2298.7–2958.0 | 16,143,464 | 65.5 | 67.9 |
| endgame | time | baseline | 12 | 3000.01 | 3000.0–3000.0 | 17,562,584 | 65.5 | 67.9 |
| endgame | time | pvs | 12 | 3000.01 | 3000.0–3000.0 | 18,313,993 | 65.5 | 67.9 |
| tactics | depth | baseline | 7 | 217.92 | 213.4–219.6 | 2,291,642 | 4.2 | 6.5 |
| tactics | depth | pvs | 7 | 224.24 | 215.2–279.4 | 2,266,483 | 4.2 | 6.5 |
| tactics | time | baseline | 8 | 3000.07 | 3000.1–3000.5 | 17,319,580 | 45.9 | 48.3 |
| tactics | time | pvs | 8 | 3000.09 | 3000.1–3000.2 | 16,790,579 | 44.1 | 46.6 |
| pawns | depth | baseline | 12 | 128.79 | 128.4–133.3 | 811,704 | 4.2 | 6.5 |
| pawns | depth | pvs | 12 | 125.52 | 124.4–217.2 | 842,858 | 4.2 | 6.5 |
| pawns | time | baseline | 16 | 3000.00 | 3000.0–3000.0 | 15,400,336 | 65.5 | 68.0 |
| pawns | time | pvs | 16 | 3000.00 | 3000.0–3000.0 | 13,431,855 | 65.5 | 68.0 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9, middlegame 7, endgame 12, tactics 7, pawns 12.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.
