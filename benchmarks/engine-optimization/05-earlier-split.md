# Measured results

Fresh processes, cold caches; medians and ranges. Evaluation and fixed-depth scores must agree for optimization studies.

| Position | Threads | Mode / ms | Variant | Depth range | Score range | Best moves | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---|---|---:|---:|
| opening | 1 | depth / 0 | optimized | 8–8 | 0–0 | b1c3 | 763.78 (753.916–769.259) | 8.16 | 10.22 |
| opening | 1 | depth / 0 | baseline | 8–8 | 0–0 | b1c3 | 760.30 (750.125–766.209) | 8.16 | 10.24 |
| opening | 6 | depth / 0 | baseline | 8–8 | 0–0 | b1c3 | 273.64 (264.338–333.38) | 8.42 | 12.76 |
| opening | 6 | depth / 0 | optimized | 8–8 | 0–0 | b1c3 | 281.94 (257.824–301.429) | 8.42 | 12.76 |
| after_e4 | 1 | depth / 0 | baseline | 7–7 | 12–12 | d7d5 | 449.62 (440.797–452.456) | 4.21 | 6.28 |
| after_e4 | 1 | depth / 0 | optimized | 7–7 | 12–12 | d7d5 | 446.96 (440.756–451.756) | 4.21 | 6.28 |
| after_e4 | 6 | depth / 0 | baseline | 7–7 | 12–12 | d7d5 | 272.67 (268.742–298.33) | 4.21 | 8.47 |
| after_e4 | 6 | depth / 0 | optimized | 7–7 | 12–12 | d7d5 | 266.20 (252.943–279.204) | 4.21 | 8.47 |
| after_d4 | 1 | depth / 0 | optimized | 7–7 | 0–0 | g8f6 | 233.15 (230.712–238.451) | 4.21 | 6.26 |
| after_d4 | 1 | depth / 0 | baseline | 7–7 | 0–0 | g8f6 | 231.63 (230.405–237.687) | 4.21 | 6.26 |
| after_d4 | 6 | depth / 0 | baseline | 7–7 | 0–0 | g8f6 | 75.10 (71.6012–78.6633) | 4.21 | 8.45 |
| after_d4 | 6 | depth / 0 | optimized | 7–7 | 0–0 | g8f6 | 71.98 (68.5192–76.2405) | 4.21 | 8.45 |
| middlegame | 1 | depth / 0 | optimized | 6–6 | -9–-9 | d5e6 | 589.42 (582.512–624.699) | 4.21 | 6.28 |
| middlegame | 1 | depth / 0 | baseline | 6–6 | -9–-9 | d5e6 | 585.37 (582.938–591.364) | 4.21 | 6.28 |
| middlegame | 6 | depth / 0 | baseline | 6–6 | -9–-9 | d5e6 | 282.18 (279.816–310.062) | 4.21 | 8.44 |
| middlegame | 6 | depth / 0 | optimized | 6–6 | -9–-9 | d5e6 | 252.54 (249.678–271.532) | 4.21 | 8.49 |
| closed_center | 1 | depth / 0 | baseline | 6–6 | 10–10 | g1f3 | 231.97 (229.567–236.739) | 4.21 | 6.26 |
| closed_center | 1 | depth / 0 | optimized | 6–6 | 10–10 | g1f3 | 230.66 (229.278–235.079) | 4.21 | 6.26 |
| closed_center | 6 | depth / 0 | baseline | 6–6 | 10–10 | g1f3 | 105.76 (98.2269–124.98) | 4.21 | 8.40 |
| closed_center | 6 | depth / 0 | optimized | 6–6 | 10–10 | g1f3 | 93.33 (82.2281–120.99) | 4.21 | 8.45 |
| endgame | 1 | depth / 0 | baseline | 10–10 | 11–11 | b4f4 | 962.69 (956.177–981.016) | 13.22 | 15.29 |
| endgame | 1 | depth / 0 | optimized | 10–10 | 11–11 | b4f4 | 961.85 (959–969.331) | 13.22 | 15.29 |
| endgame | 6 | depth / 0 | baseline | 10–10 | 11–11 | b4f4 | 459.99 (426.722–516.414) | 16.84 | 21.14 |
| endgame | 6 | depth / 0 | optimized | 10–10 | 11–11 | b4f4 | 480.73 (422.522–495.243) | 16.84 | 21.18 |
| tactics | 1 | depth / 0 | baseline | 6–6 | 491–491 | d7c8q | 217.75 (216.434–221.216) | 4.21 | 6.28 |
| tactics | 1 | depth / 0 | optimized | 6–6 | 491–491 | d7c8q | 217.13 (215.318–221.878) | 4.21 | 6.28 |
| tactics | 6 | depth / 0 | optimized | 6–6 | 491–491 | d7c8q | 59.44 (58.0123–61.6874) | 4.21 | 8.45 |
| tactics | 6 | depth / 0 | baseline | 6–6 | 491–491 | d7c8q | 63.02 (62.0552–66.131) | 4.21 | 8.40 |
| king_attack | 1 | depth / 0 | optimized | 6–6 | 59–59 | c3d5 | 874.20 (867.625–886.033) | 4.21 | 6.28 |
| king_attack | 1 | depth / 0 | baseline | 6–6 | 59–59 | c3d5 | 875.70 (865.89–898.483) | 4.21 | 6.28 |
| king_attack | 6 | depth / 0 | baseline | 6–6 | 59–59 | c3d5 | 312.66 (304.875–324.556) | 4.21 | 8.42 |
| king_attack | 6 | depth / 0 | optimized | 6–6 | 59–59 | c3d5 | 268.97 (263.719–276.077) | 4.21 | 8.45 |
| pawns | 1 | depth / 0 | baseline | 10–10 | -2–-2 | g3f3 | 64.82 (64.4155–68.2469) | 4.21 | 6.26 |
| pawns | 1 | depth / 0 | optimized | 10–10 | -2–-2 | g3f3 | 64.52 (64.4081–66.2328) | 4.21 | 6.26 |
| pawns | 6 | depth / 0 | baseline | 10–10 | -2–-2 | g3f3 | 32.09 (30.7032–33.0563) | 4.21 | 8.47 |
| pawns | 6 | depth / 0 | optimized | 10–10 | -2–-2 | g3f3 | 33.54 (30.5828–33.7836) | 4.21 | 8.54 |
