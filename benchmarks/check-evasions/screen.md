# Measurements

Fresh processes, seeded shuffled order, equal 100 MB total budget. Ponder: warmup then 2 s reply.

| Position | Threads | Mode | Variant | Depth range | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---:|---:|
| opening | 1 | depth | baseline | 8–8 | 1055.40 (880.85–1331.56) | 8.42 | 10.55 |
| opening | 1 | depth | evasions | 8–8 | 1013.82 (833.65–1257.55) | 8.42 | 10.52 |
| after_e4 | 1 | depth | baseline | 7–7 | 310.17 (260.85–394.51) | 4.21 | 6.31 |
| after_e4 | 1 | depth | evasions | 7–7 | 259.93 (241.19–355.35) | 4.21 | 6.62 |
| after_d4 | 1 | depth | baseline | 7–7 | 356.53 (253.73–383.81) | 4.21 | 6.32 |
| after_d4 | 1 | depth | evasions | 7–7 | 347.01 (252.32–408.69) | 4.21 | 6.29 |
| middlegame | 1 | depth | baseline | 6–6 | 704.86 (540.73–936.64) | 4.21 | 6.34 |
| middlegame | 1 | depth | evasions | 6–6 | 645.29 (512.64–755.52) | 4.21 | 6.29 |
| closed_center | 1 | depth | evasions | 6–6 | 229.20 (175.04–263.00) | 4.21 | 6.28 |
| closed_center | 1 | depth | baseline | 6–6 | 268.51 (187.34–293.77) | 4.21 | 6.28 |
| endgame | 1 | depth | baseline | 10–10 | 1279.02 (1039.28–1466.10) | 15.86 | 17.94 |
| endgame | 1 | depth | evasions | 10–10 | 1193.44 (982.55–1346.06) | 15.86 | 17.94 |
| tactics | 1 | depth | evasions | 6–6 | 201.13 (190.25–306.47) | 4.21 | 6.31 |
| tactics | 1 | depth | baseline | 6–6 | 264.31 (223.84–331.66) | 4.21 | 6.31 |
| relationships | 1 | depth | evasions | 10–10 | 66.55 (66.03–68.21) | 4.21 | 6.26 |
| relationships | 1 | depth | baseline | 10–10 | 66.71 (66.48–71.15) | 4.21 | 6.28 |
