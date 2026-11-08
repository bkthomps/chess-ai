# Tactical aspiration follow-up: controls

Baseline is PVS at `98d2f0d`; archived experimental aspiration uses ±64 centipawns. The application uses PVS without aspiration. Fresh processes, cold caches, 100 MB total budget. Timing stages discard one warm-up group; diagnostic counters need no warm-up. Variant names refer to the archived experiment. No production source changes.

Positive paired cost means slower than the baseline in the same repetition. Intervals are descriptive percentile bootstrap intervals over pairs (10,000 resamples); adjacent measurements may be correlated and results are specific to this machine/build.

| Stage | Variant | Repetitions | Median ms | Range ms | Paired cost % | Bootstrap 95% range % | Nodes, median | Completed depths |
|---|---|---:|---:|---:|---:|---:|---:|---|
| serial-depth7 | aspiration | 9 | 659.71 | 653.8–675.1 | +0.23 | -2.29 to +1.39 | 2,195,726 | 7: 9 |
| serial-depth7 | baseline | 9 | 660.14 | 652.3–726.6 | +0.00 | +0.00 to +0.00 | 2,195,781 | 7: 9 |
| serial-depth7 | current_full | 9 | 665.41 | 655.6–678.0 | +0.46 | -0.97 to +1.56 | 2,195,781 | 7: 9 |
| serial-depth7 | margin128 | 9 | 655.48 | 652.9–746.2 | -0.52 | -1.61 to +0.36 | 2,195,731 | 7: 9 |
| serial-depth7 | margin32 | 9 | 828.37 | 824.8–837.5 | +25.31 | +22.77 to +27.01 | 2,511,051 | 7: 9 |
| parallel-depth7 | aspiration | 9 | 192.10 | 183.9–205.1 | -4.31 | -8.34 to +1.06 | 2,255,645 | 7: 9 |
| parallel-depth7 | baseline | 9 | 200.59 | 192.5–212.2 | +0.00 | +0.00 to +0.00 | 2,259,573 | 7: 9 |
| parallel-depth7 | current_full | 9 | 193.36 | 188.7–230.0 | -1.76 | -4.18 to +0.11 | 2,252,571 | 7: 9 |
| parallel-depth7 | margin128 | 9 | 196.64 | 189.8–238.2 | -0.88 | -2.78 to +1.28 | 2,256,907 | 7: 9 |
| parallel-depth7 | margin32 | 9 | 252.61 | 246.5–300.0 | +28.14 | +22.14 to +41.35 | 2,586,120 | 7: 9 |
