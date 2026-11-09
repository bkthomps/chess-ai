# Measured results

Fresh processes, cold caches; medians and ranges. Evaluation and fixed-depth scores must agree for optimization studies.

| Position | Threads | Mode / ms | Variant | Depth range | Score range | Best moves | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---|---|---:|---:|
| opening | 1 | depth / 0 | optimized | 8–8 | 0–0 | b1c3 | 804.13 (788.603–823.554) | 8.16 | 9.90 |
| opening | 1 | depth / 0 | baseline | 8–8 | 0–0 | b1c3 | 798.83 (790.895–805.926) | 8.16 | 9.91 |
| after_e4 | 1 | depth / 0 | baseline | 7–7 | 12–12 | d7d5 | 466.94 (460.921–478.945) | 4.21 | 5.95 |
| after_e4 | 1 | depth / 0 | optimized | 7–7 | 12–12 | d7d5 | 471.91 (460.778–478.65) | 4.21 | 5.93 |
| after_d4 | 1 | depth / 0 | baseline | 7–7 | 0–0 | g8f6 | 248.31 (241.307–261.888) | 4.21 | 5.93 |
| after_d4 | 1 | depth / 0 | optimized | 7–7 | 0–0 | g8f6 | 247.32 (243.228–265.445) | 4.21 | 5.91 |
| middlegame | 1 | depth / 0 | baseline | 6–6 | -9–-9 | d5e6 | 612.71 (602.101–617.476) | 4.21 | 5.95 |
| middlegame | 1 | depth / 0 | optimized | 6–6 | -9–-9 | d5e6 | 612.80 (603.512–618.585) | 4.21 | 5.93 |
| closed_center | 1 | depth / 0 | baseline | 6–6 | 10–10 | g1f3 | 247.98 (242.755–253.912) | 4.21 | 5.93 |
| closed_center | 1 | depth / 0 | optimized | 6–6 | 10–10 | g1f3 | 244.89 (242.762–252.038) | 4.21 | 5.91 |
| endgame | 1 | depth / 0 | optimized | 10–10 | 11–11 | b4f4 | 1060.56 (1045.32–1068.79) | 13.22 | 14.96 |
| endgame | 1 | depth / 0 | baseline | 10–10 | 11–11 | b4f4 | 1056.02 (1039.03–1070.52) | 13.22 | 14.97 |
| tactics | 1 | depth / 0 | optimized | 6–6 | 491–491 | d7c8q | 231.15 (228.494–252.178) | 4.21 | 5.93 |
| tactics | 1 | depth / 0 | baseline | 6–6 | 491–491 | d7c8q | 233.73 (226.734–242.715) | 4.21 | 5.95 |
| pawns | 1 | depth / 0 | baseline | 10–10 | -2–-2 | g3f3 | 69.91 (69.6412–78.1338) | 4.21 | 5.93 |
| pawns | 1 | depth / 0 | optimized | 10–10 | -2–-2 | g3f3 | 71.57 (69.8977–74.211) | 4.21 | 5.91 |
