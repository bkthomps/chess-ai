# Parallel load balancing measurements

Baseline `ac1d2b3`. 5 positions, 5 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 9 | 1489.32 | 1386.2–1561.7 | 10,124,497 | 21.1 | 23.5 |
| opening | depth | balanced | 9 | 802.49 | 760.4–864.1 | 9,617,464 | 21.3 | 23.7 |
| opening | time | baseline | 9 | 3000.01 | 3000.0–3000.0 | 14,934,912 | 33.7 | 36.0 |
| opening | time | balanced | 9 | 3000.08 | 3000.1–3000.4 | 31,953,733 | 65.5 | 68.0 |
| middlegame | depth | baseline | 7 | 813.59 | 808.3–821.0 | 3,234,872 | 4.2 | 6.5 |
| middlegame | depth | balanced | 7 | 485.78 | 484.4–495.7 | 3,250,459 | 4.2 | 6.6 |
| middlegame | time | baseline | 8 | 3000.01 | 3000.0–3000.0 | 12,086,189 | 33.7 | 36.1 |
| middlegame | time | balanced | 8 | 3000.08 | 3000.1–3000.1 | 20,029,422 | 54.9 | 57.4 |
| endgame | depth | baseline | 12 | 2139.68 | 2094.3–2215.6 | 16,113,399 | 65.5 | 67.9 |
| endgame | depth | balanced | 12 | 1840.09 | 1760.4–1992.5 | 15,458,304 | 65.5 | 67.9 |
| endgame | time | baseline | 12 | 3000.01 | 3000.0–3000.0 | 18,362,587 | 65.5 | 67.9 |
| endgame | time | balanced | 12 | 3000.04 | 3000.0–3000.1 | 20,788,881 | 65.5 | 67.9 |
| tactics | depth | baseline | 7 | 195.20 | 189.2–216.9 | 2,255,068 | 4.2 | 6.5 |
| tactics | depth | balanced | 7 | 179.19 | 177.0–214.2 | 2,253,090 | 4.2 | 6.6 |
| tactics | time | baseline | 8 | 3000.08 | 3000.1–3000.1 | 21,635,682 | 49.8 | 52.3 |
| tactics | time | balanced | 8 | 3000.07 | 3000.1–3000.1 | 26,730,385 | 55.0 | 57.5 |
| pawns | depth | baseline | 12 | 111.95 | 108.0–112.3 | 818,205 | 4.2 | 6.6 |
| pawns | depth | balanced | 12 | 73.12 | 68.4–75.3 | 810,638 | 4.2 | 6.6 |
| pawns | time | baseline | 16 | 3000.00 | 3000.0–3000.0 | 15,144,683 | 65.5 | 68.0 |
| pawns | time | balanced | 16 | 3000.04 | 3000.0–3000.1 | 25,358,236 | 65.5 | 68.1 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9, middlegame 7, endgame 12, tactics 7, pawns 12.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Compiler LTO: disabled.
