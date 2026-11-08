# Quiet-move ordering measurements

Baseline `5f76829`. 5 positions, 1 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | history | 7 | 131.60 | 131.6–131.6 | 557,967 | 4.2 | 6.1 |
| opening | depth | both | 7 | 112.46 | 112.5–112.5 | 432,666 | 4.2 | 6.1 |
| opening | depth | soft | 7 | 141.99 | 142.0–142.0 | 407,549 | 4.2 | 6.1 |
| middlegame | depth | history | 5 | 196.21 | 196.2–196.2 | 400,195 | 4.2 | 6.1 |
| middlegame | depth | both | 5 | 199.70 | 199.7–199.7 | 400,195 | 4.2 | 6.0 |
| middlegame | depth | soft | 5 | 197.14 | 197.1–197.1 | 400,195 | 4.2 | 6.1 |
| endgame | depth | history | 10 | 459.73 | 459.7–459.7 | 1,530,088 | 8.4 | 10.3 |
| endgame | depth | both | 10 | 575.66 | 575.7–575.7 | 1,815,997 | 8.4 | 10.3 |
| endgame | depth | soft | 10 | 564.80 | 564.8–564.8 | 1,783,100 | 8.4 | 10.3 |
| tactics | depth | history | 5 | 35.76 | 35.8–35.8 | 118,836 | 4.2 | 6.1 |
| tactics | depth | both | 5 | 36.27 | 36.3–36.3 | 120,730 | 4.2 | 6.1 |
| tactics | depth | soft | 5 | 36.07 | 36.1–36.1 | 120,730 | 4.2 | 6.1 |
| pawns | depth | history | 10 | 44.67 | 44.7–44.7 | 174,712 | 4.2 | 6.1 |
| pawns | depth | both | 10 | 42.12 | 42.1–42.1 | 168,370 | 4.2 | 6.1 |
| pawns | depth | soft | 10 | 41.58 | 41.6–41.6 | 165,946 | 4.2 | 6.1 |

Time-limited searches use 1 seconds. Fixed-depth targets: opening 7, middlegame 5, endgame 10, tactics 5, pawns 10.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.
