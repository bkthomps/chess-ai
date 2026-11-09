# Measured results

Fresh processes, cold caches; medians and ranges. Evaluation and fixed-depth scores must agree for optimization studies.

| Position | Threads | Mode / ms | Variant | Depth range | Score range | Best moves | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---|---|---:|---:|
| opening | 1 | depth / 0 | optimized | 8–8 | 0–0 | b1c3 | 757.79 (749.905–780.537) | 8.16 | 10.24 |
| opening | 1 | depth / 0 | baseline | 8–8 | 0–0 | b1c3 | 773.43 (764.932–783.411) | 8.16 | 9.96 |
| after_e4 | 1 | depth / 0 | baseline | 7–7 | 12–12 | d7d5 | 448.34 (444.406–451.471) | 4.21 | 6.03 |
| after_e4 | 1 | depth / 0 | optimized | 7–7 | 12–12 | d7d5 | 440.65 (432.944–455.087) | 4.21 | 6.28 |
| after_d4 | 1 | depth / 0 | baseline | 7–7 | 0–0 | g8f6 | 236.73 (232.247–250.496) | 4.21 | 6.01 |
| after_d4 | 1 | depth / 0 | optimized | 7–7 | 0–0 | g8f6 | 230.61 (227.32–243.44) | 4.21 | 6.26 |
| middlegame | 1 | depth / 0 | baseline | 6–6 | -9–-9 | d5e6 | 591.21 (586.723–608.411) | 4.21 | 6.03 |
| middlegame | 1 | depth / 0 | optimized | 6–6 | -9–-9 | d5e6 | 583.75 (575.149–589.998) | 4.21 | 6.28 |
| closed_center | 1 | depth / 0 | baseline | 6–6 | 10–10 | g1f3 | 237.60 (234.518–241.006) | 4.21 | 6.01 |
| closed_center | 1 | depth / 0 | optimized | 6–6 | 10–10 | g1f3 | 232.39 (228.353–256.278) | 4.21 | 6.26 |
| endgame | 1 | depth / 0 | optimized | 10–10 | 11–11 | b4f4 | 962.60 (953.803–1139.16) | 13.22 | 15.30 |
| endgame | 1 | depth / 0 | baseline | 10–10 | 11–11 | b4f4 | 997.72 (976.31–1084.32) | 13.22 | 15.06 |
| tactics | 1 | depth / 0 | optimized | 6–6 | 491–491 | d7c8q | 219.36 (215.792–258.339) | 4.21 | 6.28 |
| tactics | 1 | depth / 0 | baseline | 6–6 | 491–491 | d7c8q | 225.81 (222.383–345.313) | 4.21 | 6.05 |
| king_attack | 1 | depth / 0 | baseline | 6–6 | 59–59 | c3d5 | 873.82 (870.228–887.142) | 4.21 | 6.03 |
| king_attack | 1 | depth / 0 | optimized | 6–6 | 59–59 | c3d5 | 864.46 (858.279–896.511) | 4.21 | 6.28 |
| pawns | 1 | depth / 0 | optimized | 10–10 | -2–-2 | g3f3 | 64.86 (63.6195–65.4074) | 4.21 | 6.26 |
| pawns | 1 | depth / 0 | baseline | 10–10 | -2–-2 | g3f3 | 65.61 (64.9884–66.6383) | 4.21 | 6.01 |
