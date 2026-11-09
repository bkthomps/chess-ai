# Measured results

Fresh processes, cold caches; medians and ranges. Evaluation and fixed-depth scores must agree for optimization studies.

| Position | Threads | Mode / ms | Variant | Depth range | Score range | Best moves | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---|---|---:|---:|
| opening | 1 | depth / 0 | optimized | 8–8 | 0–0 | b1c3 | 770.20 (761.227–779.933) | 8.16 | 9.96 |
| opening | 1 | depth / 0 | baseline | 8–8 | 0–0 | b1c3 | 800.17 (789.15–802.6) | 8.16 | 9.91 |
| after_e4 | 1 | depth / 0 | baseline | 7–7 | 12–12 | d7d5 | 475.03 (465.097–504.905) | 4.21 | 5.95 |
| after_e4 | 1 | depth / 0 | optimized | 7–7 | 12–12 | d7d5 | 462.25 (444.133–469.2) | 4.21 | 6.03 |
| after_d4 | 1 | depth / 0 | baseline | 7–7 | 0–0 | g8f6 | 248.19 (245.72–254.329) | 4.21 | 5.93 |
| after_d4 | 1 | depth / 0 | optimized | 7–7 | 0–0 | g8f6 | 238.30 (236.413–239.751) | 4.21 | 6.01 |
| middlegame | 1 | depth / 0 | baseline | 6–6 | -9–-9 | d5e6 | 611.49 (608.941–619.269) | 4.21 | 5.95 |
| middlegame | 1 | depth / 0 | optimized | 6–6 | -9–-9 | d5e6 | 597.52 (594.584–602.279) | 4.21 | 6.03 |
| closed_center | 1 | depth / 0 | baseline | 6–6 | 10–10 | g1f3 | 250.12 (247.584–258.656) | 4.21 | 5.93 |
| closed_center | 1 | depth / 0 | optimized | 6–6 | 10–10 | g1f3 | 238.68 (236.436–254.019) | 4.21 | 6.01 |
| endgame | 1 | depth / 0 | optimized | 10–10 | 11–11 | b4f4 | 996.88 (982.45–1205.97) | 13.22 | 15.06 |
| endgame | 1 | depth / 0 | baseline | 10–10 | 11–11 | b4f4 | 1068.81 (1060.81–1288.02) | 13.22 | 14.97 |
| tactics | 1 | depth / 0 | optimized | 6–6 | 491–491 | d7c8q | 224.54 (222.682–224.897) | 4.21 | 6.03 |
| tactics | 1 | depth / 0 | baseline | 6–6 | 491–491 | d7c8q | 232.02 (230.407–234.404) | 4.21 | 5.95 |
| king_attack | 1 | depth / 0 | baseline | 6–6 | 59–59 | c3d5 | 895.07 (892.164–915.589) | 4.21 | 5.95 |
| king_attack | 1 | depth / 0 | optimized | 6–6 | 59–59 | c3d5 | 872.17 (869.287–902.653) | 4.21 | 6.03 |
| pawns | 1 | depth / 0 | optimized | 10–10 | -2–-2 | g3f3 | 65.34 (64.5495–66.2799) | 4.21 | 6.01 |
| pawns | 1 | depth / 0 | baseline | 10–10 | -2–-2 | g3f3 | 71.35 (70.5259–71.704) | 4.21 | 5.93 |
