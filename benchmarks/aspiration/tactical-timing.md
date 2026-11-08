# Tactical aspiration follow-up: timing

Baseline is PVS at `98d2f0d`; archived experimental aspiration uses ±64 centipawns. The application uses PVS without aspiration. Fresh processes, cold caches, 100 MB total budget. Timing stages discard one warm-up group; diagnostic counters need no warm-up. Variant names refer to the archived experiment. No production source changes.

Positive paired cost means slower than the baseline in the same repetition. Intervals are descriptive percentile bootstrap intervals over pairs (10,000 resamples); adjacent measurements may be correlated and results are specific to this machine/build.

| Stage | Variant | Repetitions | Median ms | Range ms | Paired cost % | Bootstrap 95% range % | Nodes, median | Completed depths |
|---|---|---:|---:|---:|---:|---:|---:|---|
| serial-depth7 | aspiration | 31 | 658.00 | 644.8–764.6 | -0.36 | -0.62 to +0.35 | 2,195,726 | 7: 31 |
| serial-depth7 | baseline | 31 | 657.85 | 644.4–916.9 | +0.00 | +0.00 to +0.00 | 2,195,781 | 7: 31 |
| parallel-depth7 | aspiration | 31 | 190.17 | 184.2–194.4 | -1.45 | -2.14 to +0.08 | 2,254,386 | 7: 31 |
| parallel-depth7 | baseline | 31 | 192.06 | 183.4–200.5 | +0.00 | +0.00 to +0.00 | 2,259,146 | 7: 31 |
| serial-depth8 | aspiration | 9 | 3505.43 | 3472.1–3565.9 | +0.24 | -0.06 to +1.51 | 7,477,148 | 8: 9 |
| serial-depth8 | baseline | 9 | 3491.54 | 3457.8–3512.9 | +0.00 | +0.00 to +0.00 | 7,477,203 | 8: 9 |
| parallel-depth8 | aspiration | 9 | 1181.05 | 1165.2–1218.0 | +0.41 | -0.94 to +1.70 | 7,801,364 | 8: 9 |
| parallel-depth8 | baseline | 9 | 1180.09 | 1157.6–1355.2 | +0.00 | +0.00 to +0.00 | 7,835,280 | 8: 9 |
| serial-time3 | aspiration | 7 | 3000.01 | 3000.0–3000.0 | -0.00 | -0.00 to +0.00 | 6,503,136 | 7: 7 |
| serial-time3 | baseline | 7 | 3000.00 | 3000.0–3000.0 | +0.00 | +0.00 to +0.00 | 6,490,880 | 7: 7 |
| parallel-time3 | aspiration | 11 | 3000.07 | 3000.1–3000.1 | -0.00 | -0.00 to +0.00 | 21,457,751 | 8: 11 |
| parallel-time3 | baseline | 11 | 3000.07 | 3000.1–3000.1 | +0.00 | +0.00 to +0.00 | 22,137,104 | 8: 11 |
