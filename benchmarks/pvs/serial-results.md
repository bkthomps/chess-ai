# Principal Variation Search measurements

Baseline `6765813`. 5 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 9 | 2944.28 | 2942.2–3072.9 | 10,096,282 | 21.1 | 22.9 |
| opening | depth | pvs | 9 | 2456.22 | 2446.4–2459.5 | 8,436,304 | 17.4 | 19.2 |
| middlegame | depth | baseline | 7 | 1992.49 | 1987.5–1995.7 | 4,222,106 | 4.2 | 6.0 |
| middlegame | depth | pvs | 7 | 1377.24 | 1377.1–1377.3 | 3,223,093 | 4.2 | 6.0 |
| endgame | depth | baseline | 12 | 5638.39 | 5605.8–5721.1 | 16,343,965 | 78.6 | 80.5 |
| endgame | depth | pvs | 12 | 5455.55 | 4984.8–5673.9 | 15,400,189 | 76.5 | 78.4 |
| tactics | depth | baseline | 7 | 721.52 | 687.0–970.9 | 2,230,440 | 4.2 | 6.0 |
| tactics | depth | pvs | 7 | 895.41 | 669.3–976.0 | 2,195,781 | 4.2 | 6.0 |
| pawns | depth | baseline | 12 | 207.89 | 205.4–209.8 | 851,960 | 4.2 | 6.0 |
| pawns | depth | pvs | 12 | 187.32 | 186.5–189.4 | 760,851 | 4.2 | 6.0 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 9, middlegame 7, endgame 12, tactics 7, pawns 12.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.
