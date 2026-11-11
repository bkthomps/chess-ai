# Measurements

Fresh processes, seeded shuffled order, equal 100 MB total budget. Ponder: warmup then 2 s reply.

| Position | Threads | Mode | Variant | Depth range | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---:|---:|
| opening | 1 | ponder | baseline | 8–8 | 2000.02 (2000.00–2000.02) | 16.84 | 18.94 |
| opening | 1 | ponder | evasions | 8–8 | 2000.01 (2000.00–2000.02) | 16.84 | 18.92 |
| opening | 6 | ponder | baseline | 8–9 | 2000.01 (2000.00–2000.07) | 42.11 | 46.53 |
| opening | 6 | ponder | evasions | 9–9 | 2000.01 (2000.01–2000.01) | 44.87 | 49.18 |
| middlegame | 1 | ponder | baseline | 6–7 | 2000.01 (2000.00–2000.02) | 8.42 | 10.52 |
| middlegame | 1 | ponder | evasions | 7–7 | 2000.01 (2000.00–2000.04) | 8.42 | 10.53 |
| middlegame | 6 | ponder | evasions | 7–7 | 2000.27 (2000.08–2002.78) | 25.00 | 29.43 |
| middlegame | 6 | ponder | baseline | 7–7 | 2000.46 (2000.09–2002.24) | 25.26 | 29.70 |
| endgame | 1 | ponder | evasions | 10–10 | 2000.01 (2000.00–2000.02) | 28.16 | 30.23 |
| endgame | 1 | ponder | baseline | 10–10 | 2000.00 (2000.00–2000.01) | 29.28 | 31.36 |
| endgame | 6 | ponder | evasions | 11–11 | 2000.08 (2000.03–2001.75) | 65.46 | 69.93 |
| endgame | 6 | ponder | baseline | 11–11 | 2000.82 (2000.06–2003.31) | 65.46 | 69.91 |
