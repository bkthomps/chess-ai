# Measured results

Fresh processes, cold caches; medians and ranges. All equal-depth scores must agree between variants.

| Position | Threads | Mode / ms | Variant | Depth range | Score range | Best moves | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---|---|---:|---:|
| opening | 1 | depth / 0 | 02-bitmasks | 8–8 | 0–0 | b1c3 | 682.73 (671.786–721.599) | 5.07 | 6.83 |
| opening | 1 | depth / 0 | 01-lazy | 8–8 | 0–0 | b1c3 | 756.06 (755.41–782.869) | 5.07 | 6.82 |
| after_e4 | 1 | depth / 0 | 01-lazy | 7–7 | 11–11 | b8c6 | 350.38 (348.784–378.015) | 4.21 | 5.95 |
| after_e4 | 1 | depth / 0 | 02-bitmasks | 7–7 | 11–11 | b8c6 | 305.54 (304.699–318.348) | 4.21 | 5.95 |
| after_d4 | 1 | depth / 0 | 02-bitmasks | 7–7 | 0–0 | g8f6 | 233.30 (232.599–245.984) | 4.21 | 5.93 |
| after_d4 | 1 | depth / 0 | 01-lazy | 7–7 | 0–0 | g8f6 | 268.93 (267.01–281.321) | 4.21 | 5.93 |
| middlegame | 1 | depth / 0 | 01-lazy | 6–6 | -7–-7 | d5e6 | 700.40 (692.421–711.76) | 4.21 | 5.95 |
| middlegame | 1 | depth / 0 | 02-bitmasks | 6–6 | -7–-7 | d5e6 | 612.98 (611.486–623.939) | 4.21 | 5.95 |
| endgame | 1 | depth / 0 | 02-bitmasks | 10–10 | 20–20 | b4f4 | 749.98 (746.035–751.165) | 8.42 | 10.16 |
| endgame | 1 | depth / 0 | 01-lazy | 10–10 | 20–20 | b4f4 | 822.94 (822.397–825.819) | 8.42 | 10.16 |
| tactics | 1 | depth / 0 | 01-lazy | 6–6 | 481–481 | d7c8q | 270.16 (266.72–274.129) | 4.21 | 5.95 |
| tactics | 1 | depth / 0 | 02-bitmasks | 6–6 | 481–481 | d7c8q | 242.43 (240.862–243.609) | 4.21 | 5.95 |
| pawns | 1 | depth / 0 | 01-lazy | 10–10 | 28–28 | g3f3 | 40.60 (40.4995–41.8749) | 4.21 | 5.93 |
| pawns | 1 | depth / 0 | 02-bitmasks | 10–10 | 28–28 | g3f3 | 39.44 (38.604–39.9755) | 4.21 | 5.93 |
