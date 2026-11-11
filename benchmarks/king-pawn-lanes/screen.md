# Measurements

Fresh processes, seeded shuffled order, equal 100 MB total budget. Ponder: warmup then 2 s reply.

| Position | Threads | Mode | Variant | Depth range | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---:|---:|
| opening | 1 | depth | baseline | 8–8 | 1109.98 (858.27–1609.47) | 8.42 | 10.55 |
| opening | 1 | depth | lanes | 8–8 | 1190.91 (858.06–1443.88) | 8.42 | 10.53 |
| after_e4 | 1 | depth | baseline | 7–7 | 399.03 (254.76–456.08) | 4.21 | 6.31 |
| after_e4 | 1 | depth | lanes | 7–7 | 333.26 (271.04–435.44) | 4.21 | 6.29 |
| after_d4 | 1 | depth | baseline | 7–7 | 356.37 (252.34–446.04) | 4.21 | 6.32 |
| after_d4 | 1 | depth | lanes | 7–7 | 371.72 (251.81–438.40) | 4.21 | 6.31 |
| middlegame | 1 | depth | baseline | 6–6 | 669.08 (544.99–828.11) | 4.21 | 6.31 |
| middlegame | 1 | depth | lanes | 6–6 | 680.86 (535.11–760.79) | 4.21 | 6.32 |
| closed_center | 1 | depth | lanes | 6–6 | 188.85 (185.61–281.97) | 4.21 | 6.31 |
| closed_center | 1 | depth | baseline | 6–6 | 188.80 (186.85–320.47) | 4.21 | 6.28 |
| endgame | 1 | depth | baseline | 10–10 | 1491.80 (1174.56–1653.90) | 15.86 | 17.96 |
| endgame | 1 | depth | lanes | 10–10 | 1355.15 (1123.81–1646.62) | 15.86 | 18.27 |
| tactics | 1 | depth | lanes | 6–6 | 335.98 (223.56–377.28) | 4.21 | 6.31 |
| tactics | 1 | depth | baseline | 6–6 | 319.84 (226.85–378.07) | 4.21 | 6.31 |
| relationships | 1 | depth | lanes | 10–10 | 68.06 (66.82–112.14) | 4.21 | 6.29 |
| relationships | 1 | depth | baseline | 10–10 | 68.06 (66.74–105.21) | 4.21 | 6.28 |
