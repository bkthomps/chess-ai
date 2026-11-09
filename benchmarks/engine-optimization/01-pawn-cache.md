# Measured results

Fresh processes, cold caches; medians and ranges. Evaluation and fixed-depth scores must agree for optimization studies.

| Position | Threads | Mode / ms | Variant | Depth range | Score range | Best moves | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---|---|---:|---:|
| opening | 1 | depth / 0 | optimized | 8–8 | 0–0 | b1c3 | 794.08 (783.385–804.298) | 8.16 | 9.91 |
| opening | 1 | depth / 0 | baseline | 8–8 | 0–0 | b1c3 | 794.82 (785.585–806.94) | 8.16 | 9.93 |
| after_e4 | 1 | depth / 0 | baseline | 7–7 | 12–12 | d7d5 | 471.75 (462.461–478.538) | 4.21 | 5.95 |
| after_e4 | 1 | depth / 0 | optimized | 7–7 | 12–12 | d7d5 | 460.60 (459.186–473.644) | 4.21 | 5.96 |
| after_d4 | 1 | depth / 0 | baseline | 7–7 | 0–0 | g8f6 | 243.18 (241.353–251.188) | 4.21 | 5.93 |
| after_d4 | 1 | depth / 0 | optimized | 7–7 | 0–0 | g8f6 | 243.97 (240.029–250.403) | 4.21 | 5.95 |
| middlegame | 1 | depth / 0 | baseline | 6–6 | -9–-9 | d5e6 | 614.33 (606.473–635.005) | 4.21 | 5.95 |
| middlegame | 1 | depth / 0 | optimized | 6–6 | -9–-9 | d5e6 | 607.20 (596.069–610.906) | 4.21 | 5.96 |
| closed_center | 1 | depth / 0 | baseline | 6–6 | 10–10 | g1f3 | 244.47 (242.487–248.445) | 4.21 | 5.93 |
| closed_center | 1 | depth / 0 | optimized | 6–6 | 10–10 | g1f3 | 244.00 (241.563–251.306) | 4.21 | 5.95 |
| endgame | 1 | depth / 0 | optimized | 10–10 | 11–11 | b4f4 | 1035.04 (1033–1056.41) | 13.22 | 14.99 |
| endgame | 1 | depth / 0 | baseline | 10–10 | 11–11 | b4f4 | 1051.01 (1045.48–1054.6) | 13.22 | 14.97 |
| tactics | 1 | depth / 0 | optimized | 6–6 | 491–491 | d7c8q | 225.29 (224.185–233.854) | 4.21 | 5.96 |
| tactics | 1 | depth / 0 | baseline | 6–6 | 491–491 | d7c8q | 230.19 (225.702–237.221) | 4.21 | 5.95 |
| pawns | 1 | depth / 0 | baseline | 10–10 | -2–-2 | g3f3 | 69.40 (69.1147–73.6057) | 4.21 | 5.93 |
| pawns | 1 | depth / 0 | optimized | 10–10 | -2–-2 | g3f3 | 68.55 (68.224–72.2854) | 4.21 | 5.95 |
