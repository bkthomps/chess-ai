# Parallel load balancing measurements

Baseline `ac1d2b3`. 5 positions, 3 fresh-process repetitions, 100 MB total RAM budget. Fixed-depth times include initialization; time/ponder times report the measured computer search. Memory is peak RSS, excluding the browser.

| Position | Mode | Variant | Depth | Time ms, median | Time range ms | Nodes, median | Cache MB | Peak RSS MB |
|---|---|---|---:|---:|---:|---:|---:|---:|
| opening | time | baseline | 9 | 5000.12 | 5000.1–5000.1 | 31,115,183 | 65.5 | 67.9 |
| opening | time | balanced | 9 | 5000.05 | 5000.0–5000.1 | 43,207,763 | 65.5 | 68.0 |
| middlegame | time | baseline | 8 | 5000.02 | 5000.0–5000.0 | 16,860,839 | 41.5 | 43.9 |
| middlegame | time | balanced | 8 | 5000.04 | 5000.0–5000.1 | 43,827,252 | 65.5 | 68.0 |
| endgame | time | baseline | 12 | 5000.07 | 5000.1–5000.1 | 39,098,007 | 65.5 | 67.9 |
| endgame | time | balanced | 12 | 5000.05 | 5000.0–5000.1 | 46,783,641 | 65.5 | 68.0 |
| tactics | time | baseline | 9 | 5000.01 | 5000.0–5000.0 | 43,549,748 | 65.5 | 67.9 |
| tactics | time | balanced | 9 | 5000.08 | 5000.1–5000.1 | 46,249,657 | 65.5 | 68.0 |
| pawns | time | baseline | 17 | 5000.01 | 5000.0–5000.1 | 25,495,276 | 65.5 | 68.0 |
| pawns | time | balanced | 17 | 5000.01 | 5000.0–5000.0 | 33,312,827 | 65.5 | 68.1 |

Time-limited searches use 5 seconds. Fixed-depth targets: opening 9, middlegame 7, endgame 12, tactics 7, pawns 12.
Fixed-depth scores must agree across all variants. Equally valued best moves may differ. Parallel scheduling can change node counts and timing; single-thread trials isolate the algorithm more closely.

Compiler LTO: disabled.
