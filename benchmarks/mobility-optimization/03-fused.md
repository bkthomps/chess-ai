# Measured results

Fresh processes, cold caches; medians and ranges. All equal-depth scores must agree between variants.

| Position | Threads | Mode / ms | Variant | Depth range | Score range | Best moves | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---|---|---:|---:|
| opening | 1 | depth / 0 | 03-fused | 8–8 | 0–0 | b1c3 | 623.63 (600.503–659.298) | 5.07 | 6.82 |
| opening | 1 | depth / 0 | 02-bitmasks | 8–8 | 0–0 | b1c3 | 674.51 (673.105–771.495) | 5.07 | 6.82 |
| after_e4 | 1 | depth / 0 | 02-bitmasks | 7–7 | 11–11 | b8c6 | 306.25 (305.355–308.115) | 4.21 | 5.95 |
| after_e4 | 1 | depth / 0 | 03-fused | 7–7 | 11–11 | b8c6 | 274.16 (272.976–276.635) | 4.21 | 5.95 |
| after_d4 | 1 | depth / 0 | 03-fused | 7–7 | 0–0 | g8f6 | 210.09 (208.325–212.821) | 4.21 | 5.93 |
| after_d4 | 1 | depth / 0 | 02-bitmasks | 7–7 | 0–0 | g8f6 | 235.71 (234.342–236.24) | 4.21 | 5.93 |
| middlegame | 1 | depth / 0 | 02-bitmasks | 6–6 | -7–-7 | d5e6 | 611.53 (609.498–614.983) | 4.21 | 5.95 |
| middlegame | 1 | depth / 0 | 03-fused | 6–6 | -7–-7 | d5e6 | 554.78 (550.877–557.442) | 4.21 | 5.95 |
| endgame | 1 | depth / 0 | 03-fused | 10–10 | 20–20 | b4f4 | 696.78 (695.331–703.294) | 8.42 | 10.16 |
| endgame | 1 | depth / 0 | 02-bitmasks | 10–10 | 20–20 | b4f4 | 750.79 (746.842–754.751) | 8.42 | 10.16 |
| tactics | 1 | depth / 0 | 02-bitmasks | 6–6 | 481–481 | d7c8q | 244.60 (243.236–255.488) | 4.21 | 5.95 |
| tactics | 1 | depth / 0 | 03-fused | 6–6 | 481–481 | d7c8q | 231.53 (221.321–252.252) | 4.21 | 5.95 |
| pawns | 1 | depth / 0 | 02-bitmasks | 10–10 | 28–28 | g3f3 | 42.69 (39.7341–44.6895) | 4.21 | 5.93 |
| pawns | 1 | depth / 0 | 03-fused | 10–10 | 28–28 | g3f3 | 43.06 (41.1841–45.971) | 4.21 | 5.96 |
