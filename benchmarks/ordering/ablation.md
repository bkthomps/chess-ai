# Quiet-move ordering measurements

Baseline `5f76829`. 5 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | depth | baseline | 8 | 2051.27 | 2046.4–2082.0 | 7,442,231 | 16.8 | 18.7 |
| opening | depth | killers | 8 | 651.60 | 646.9–719.2 | 2,086,290 | 8.4 | 10.3 |
| opening | depth | history | 8 | 733.53 | 732.0–737.8 | 2,180,718 | 8.4 | 10.3 |
| opening | depth | both | 8 | 533.55 | 532.6–538.8 | 1,620,631 | 4.3 | 6.2 |
| middlegame | depth | baseline | 6 | 532.93 | 530.9–545.6 | 1,085,829 | 4.2 | 6.1 |
| middlegame | depth | killers | 6 | 532.66 | 532.1–587.8 | 1,083,051 | 4.2 | 6.1 |
| middlegame | depth | history | 6 | 542.94 | 542.7–545.4 | 1,072,690 | 4.2 | 6.1 |
| middlegame | depth | both | 6 | 540.55 | 537.7–545.3 | 1,072,690 | 4.2 | 6.1 |
| endgame | depth | baseline | 11 | 3352.38 | 3088.7–3389.2 | 12,259,718 | 32.3 | 34.2 |
| endgame | depth | killers | 11 | 1858.82 | 1837.2–2251.0 | 7,459,853 | 22.8 | 24.7 |
| endgame | depth | history | 11 | 1268.81 | 1253.6–1282.6 | 4,727,466 | 16.8 | 18.7 |
| endgame | depth | both | 11 | 1649.25 | 1635.0–1738.5 | 5,990,915 | 19.7 | 21.6 |
| tactics | depth | baseline | 6 | 195.19 | 193.9–196.7 | 446,668 | 4.2 | 6.1 |
| tactics | depth | killers | 6 | 197.12 | 195.0–197.4 | 433,809 | 4.2 | 6.1 |
| tactics | depth | history | 6 | 194.43 | 194.0–195.3 | 423,222 | 4.2 | 6.1 |
| tactics | depth | both | 6 | 194.99 | 190.7–197.6 | 418,924 | 4.2 | 6.1 |
| pawns | depth | baseline | 11 | 149.75 | 145.7–202.4 | 653,166 | 4.2 | 6.1 |
| pawns | depth | killers | 11 | 112.65 | 110.3–122.6 | 479,595 | 4.2 | 6.1 |
| pawns | depth | history | 11 | 94.36 | 92.2–97.9 | 421,756 | 4.2 | 6.1 |
| pawns | depth | both | 11 | 97.91 | 91.4–102.8 | 403,856 | 4.2 | 6.1 |

Time-limited searches use 3 seconds. Fixed-depth targets: opening 8, middlegame 6, endgame 11, tactics 6, pawns 11.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.
