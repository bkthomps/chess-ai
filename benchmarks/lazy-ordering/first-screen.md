# Measurements

Fresh processes, seeded shuffled order, equal 100 MB total budget. Ponder: warmup then 2 s reply.

| Position | Threads | Mode | Variant | Depth range | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---:|---:|
| opening | 1 | depth | baseline | 8–8 | 1343.27 (1104.70–1455.50) | 8.42 | 10.53 |
| opening | 1 | depth | lazy | 8–8 | 1330.78 (1075.52–1435.40) | 8.42 | 10.53 |
| after_e4 | 1 | depth | baseline | 7–7 | 373.28 (327.89–436.32) | 4.21 | 6.31 |
| after_e4 | 1 | depth | lazy | 7–7 | 345.97 (272.30–423.63) | 4.21 | 6.28 |
| after_d4 | 1 | depth | baseline | 7–7 | 358.76 (279.58–434.16) | 4.21 | 6.31 |
| after_d4 | 1 | depth | lazy | 7–7 | 368.14 (262.69–409.05) | 4.21 | 6.29 |
| middlegame | 1 | depth | lazy | 6–6 | 783.72 (728.27–891.95) | 4.21 | 6.31 |
| middlegame | 1 | depth | baseline | 6–6 | 822.82 (643.76–915.42) | 4.21 | 6.31 |
| closed_center | 1 | depth | lazy | 6–6 | 254.29 (193.38–286.30) | 4.21 | 6.26 |
| closed_center | 1 | depth | baseline | 6–6 | 259.49 (225.74–312.29) | 4.21 | 6.65 |
| endgame | 1 | depth | lazy | 10–10 | 1316.72 (1227.81–1448.50) | 15.86 | 17.92 |
| endgame | 1 | depth | baseline | 10–10 | 1274.33 (1210.90–1627.40) | 15.86 | 17.96 |
| tactics | 1 | depth | lazy | 6–6 | 270.19 (227.21–327.43) | 4.21 | 6.28 |
| tactics | 1 | depth | baseline | 6–6 | 310.43 (287.38–359.08) | 4.21 | 6.31 |
| relationships | 1 | depth | baseline | 10–10 | 80.04 (67.35–111.46) | 4.21 | 6.29 |
| relationships | 1 | depth | lazy | 10–10 | 73.06 (67.72–103.16) | 4.21 | 6.26 |
