# Search memory comparison

Baseline: `9288315`; machine: macOS-26.5.1-arm64-arm-64bit.

Budgets are decimal MB of total engine/server RAM, with cache headroom reserved for other allocations. Each run is a fresh process and cold cache. Peak RSS includes process overhead and is not the configured ceiling. Timing includes initialization. All tiers use the same CPU headroom policy.

| Position | Budget | Depth in timed search | Fixed-depth time (ms) | Cache MB, timed | Peak RSS MB, timed |
|---|---:|---:|---:|---:|---:|
| opening | original ~30 | 9 | 9314.0 | 28.0 | 32.7 |
| opening | 100 | 9 | 8668.6 | 65.5 | 67.7 |
| opening | 250 | 9 | 9290.6 | 130.7 | 133.0 |
| opening | 500 | 9 | 8676.4 | 130.8 | 132.9 |
| opening | 1000 | 9 | 8644.3 | 134.5 | 136.8 |
| middlegame | original ~30 | 8 | 1393.0 | 28.0 | 32.7 |
| middlegame | 100 | 8 | 1391.6 | 58.9 | 61.2 |
| middlegame | 250 | 8 | 1386.6 | 58.9 | 61.2 |
| middlegame | 500 | 8 | 1387.3 | 58.9 | 61.2 |
| middlegame | 1000 | 8 | 1388.2 | 58.9 | 61.2 |
| endgame | original ~30 | 12 | 1181.4 | 28.0 | 32.8 |
| endgame | 100 | 13 | 1180.5 | 65.5 | 67.8 |
| endgame | 250 | 13 | 1168.2 | 212.4 | 214.9 |
| endgame | 500 | 13 | 1176.2 | 264.9 | 267.4 |
| endgame | 1000 | 13 | 1170.3 | 264.7 | 267.2 |

Timed searches: 10 seconds; fixed targets: opening 9, middlegame 7, endgame 11.
3 repetitions per cell; medians shown. Results depend on the position and machine. Cache growth is demand-driven, so a larger ceiling may use the same amount of RAM and produce the same depth.
