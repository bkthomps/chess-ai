# Confirmation measurements

Fresh processes, cold caches, one thread, equal 100 MB total budget. Medians and ranges.

| Position | Mode | Variant | Depth range | Median ms (range) | Peak RSS MB |
|---|---|---|---|---|---:|
| opening | depth | baseline | 8–8 | 1126.94 (1123.25–1182.52) | 10.21 |
| opening | depth | files | 8–8 | 1125.59 (1122.99–1185.83) | 10.17 |
| opening | depth | shared | 8–8 | 1123.68 (1118.30–1171.63) | 10.17 |
| opening | depth | skip | 8–8 | 1129.97 (1125.90–1198.07) | 10.17 |
| opening | time | baseline | 6–6 | 100.01 (100.00–100.01) | 5.93 |
| opening | time | files | 6–6 | 100.01 (100.00–100.02) | 5.95 |
| opening | time | shared | 6–6 | 100.00 (100.00–100.01) | 5.93 |
| opening | time | skip | 6–6 | 100.00 (100.00–100.02) | 5.93 |
| middlegame | depth | baseline | 6–6 | 503.41 (501.41–530.63) | 5.95 |
| middlegame | depth | files | 6–6 | 503.93 (502.72–647.50) | 5.95 |
| middlegame | depth | shared | 6–6 | 501.98 (499.74–503.45) | 5.95 |
| middlegame | depth | skip | 6–6 | 503.95 (502.55–508.96) | 5.95 |
| middlegame | time | baseline | 3–4 | 100.01 (100.00–100.04) | 5.96 |
| middlegame | time | files | 4–4 | 100.01 (100.00–100.02) | 5.95 |
| middlegame | time | shared | 4–4 | 100.01 (100.00–100.02) | 5.95 |
| middlegame | time | skip | 4–4 | 100.01 (100.00–176.83) | 5.95 |
| endgame | depth | baseline | 10–10 | 787.16 (774.87–884.58) | 14.37 |
| endgame | depth | files | 10–10 | 792.34 (776.99–822.52) | 14.39 |
| endgame | depth | shared | 10–10 | 788.30 (778.87–831.32) | 14.37 |
| endgame | depth | skip | 10–10 | 787.70 (772.91–826.56) | 14.37 |
| endgame | time | baseline | 7–8 | 100.01 (100.00–100.01) | 5.93 |
| endgame | time | files | 7–8 | 100.01 (100.00–100.02) | 5.93 |
| endgame | time | shared | 7–8 | 100.01 (100.00–100.02) | 5.93 |
| endgame | time | skip | 7–8 | 100.00 (100.00–100.01) | 5.93 |
| relationships | depth | baseline | 10–10 | 66.11 (57.32–78.35) | 5.93 |
| relationships | depth | files | 10–10 | 66.02 (59.65–184.04) | 5.93 |
| relationships | depth | shared | 10–10 | 65.87 (58.80–109.57) | 5.93 |
| relationships | depth | skip | 10–10 | 66.10 (59.11–78.35) | 5.93 |
| relationships | time | baseline | 9–10 | 100.00 (100.00–100.01) | 5.93 |
| relationships | time | files | 10–10 | 100.00 (100.00–100.01) | 5.95 |
| relationships | time | shared | 10–10 | 100.00 (100.00–100.27) | 5.93 |
| relationships | time | skip | 10–10 | 100.00 (100.00–100.01) | 5.93 |
