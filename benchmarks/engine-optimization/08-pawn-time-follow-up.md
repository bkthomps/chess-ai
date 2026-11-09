# Measured results

Fresh processes, cold caches; medians and ranges. Evaluation and fixed-depth scores must agree for optimization studies.

| Position | Threads | Mode / ms | Variant | Depth range | Score range | Best moves | Median ms (range) | Max cache MB | Peak RSS MB |
|---|---:|---|---|---|---|---|---|---:|---:|
| pawns | 1 | time / 100 | optimized | 9–10 | -2–8 | f4f5, g3f3 | 100.01 (100.001–100.01) | 4.21 | 6.26 |
| pawns | 1 | time / 100 | baseline | 10–10 | -2–-2 | g3f3 | 100.01 (100–100.013) | 4.21 | 5.91 |
| pawns | 1 | time / 500 | baseline | 12–12 | 0–0 | f4f5 | 500.00 (500.001–500.007) | 8.42 | 10.14 |
| pawns | 1 | time / 500 | optimized | 13–13 | 0–0 | f4f5 | 500.00 (500.001–500.013) | 8.42 | 10.49 |
