# Quiet-move ordering measurements

Baseline `5f76829`. 5 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 9 | 8378.48 | 8349.4–8851.0 | 45,488,407 | 65.5 | 67.8 |
| opening | depth | history | 9 | 1618.83 | 1535.8–1813.6 | 9,347,252 | 21.4 | 23.8 |
| opening | depth | both | 9 | 1595.61 | 1550.1–1642.5 | 10,578,791 | 24.7 | 27.1 |
| opening | time | baseline | 8 | 3000.00 | 3000.0–3000.0 | 12,944,682 | 33.7 | 35.8 |
| opening | time | history | 9 | 3000.01 | 3000.0–3000.0 | 13,850,661 | 33.7 | 36.0 |
| opening | time | both | 9 | 3000.01 | 3000.0–3000.0 | 14,700,713 | 33.7 | 36.0 |
| middlegame | depth | baseline | 7 | 1356.85 | 1353.6–1372.3 | 4,210,480 | 4.2 | 6.5 |
| middlegame | depth | history | 7 | 1453.65 | 1433.6–1459.3 | 4,226,217 | 4.2 | 6.5 |
| middlegame | depth | both | 7 | 1399.86 | 1398.2–1465.2 | 4,226,897 | 4.2 | 6.6 |
| middlegame | time | baseline | 7 | 3000.04 | 3000.0–3000.1 | 12,224,939 | 33.7 | 36.0 |
| middlegame | time | history | 7 | 3000.08 | 3000.1–3000.1 | 11,132,019 | 29.4 | 31.8 |
| middlegame | time | both | 7 | 3000.08 | 3000.1–3000.1 | 11,164,990 | 29.4 | 31.8 |
| endgame | depth | baseline | 12 | 3606.95 | 3595.5–4862.9 | 34,436,416 | 65.5 | 67.8 |
| endgame | depth | history | 12 | 1966.04 | 1780.9–1968.2 | 12,612,980 | 63.7 | 66.1 |
| endgame | depth | both | 12 | 2417.69 | 2393.0–2423.7 | 16,955,572 | 65.5 | 68.0 |
| endgame | time | baseline | 11 | 3000.04 | 3000.0–3000.1 | 30,042,888 | 65.5 | 67.9 |
| endgame | time | history | 12 | 3000.01 | 3000.0–3000.0 | 16,555,610 | 65.5 | 67.9 |
| endgame | time | both | 12 | 3000.01 | 3000.0–3000.0 | 18,659,733 | 65.5 | 67.9 |
| tactics | depth | baseline | 7 | 190.95 | 188.9–193.4 | 2,224,451 | 4.2 | 6.4 |
| tactics | depth | history | 7 | 189.46 | 185.9–191.5 | 2,226,224 | 4.2 | 6.5 |
| tactics | depth | both | 7 | 206.44 | 203.6–220.3 | 2,265,433 | 4.2 | 6.5 |
| tactics | time | baseline | 8 | 3000.07 | 3000.1–3000.1 | 23,233,028 | 50.2 | 52.6 |
| tactics | time | history | 8 | 3000.08 | 3000.1–3000.1 | 24,281,854 | 52.0 | 54.5 |
| tactics | time | both | 8 | 3000.07 | 3000.1–3000.1 | 22,735,868 | 50.7 | 53.2 |
| pawns | depth | baseline | 12 | 196.79 | 196.6–200.4 | 1,482,319 | 8.4 | 10.7 |
| pawns | depth | history | 12 | 121.66 | 119.0–123.2 | 870,197 | 4.2 | 6.6 |
| pawns | depth | both | 12 | 119.21 | 114.6–124.5 | 834,286 | 4.2 | 6.6 |
| pawns | time | baseline | 14 | 3000.01 | 3000.0–3000.0 | 14,599,553 | 65.5 | 67.8 |
| pawns | time | history | 16 | 3000.01 | 3000.0–3000.0 | 15,944,219 | 65.5 | 68.0 |
| pawns | time | both | 16 | 3000.01 | 3000.0–3000.0 | 15,379,437 | 65.5 | 68.0 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9, middlegame 7, endgame 12, tactics 7, pawns 12.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.
