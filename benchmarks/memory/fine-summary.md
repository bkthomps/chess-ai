# Fine memory budget sweep

After this sweep, the application default was set to **100 MB**.

Tested 50, 75, 100, 125, 150, 200, 225, and 250 MB total RAM budgets. The application configuration was not changed. All runs use the current engine and the same six-thread limit on this eight-logical-CPU Mac. MB means 1,000,000 bytes.

Each budget received three cold ten-second searches in each of the opening, middlegame, and endgame positions, plus three cold searches to endgame depth 13: 96 measured searches total. Order was randomized within each repetition. All fixed-depth searches returned score 52 and move b4f4. Source hashes and compiler/platform information are recorded in the JSON.

## Results

| Budget MB | Median depth: opening / middle / end, 10 s | End depth 13 reached within 10 s | Median time to end depth 13, s | Time range, s | Median nodes to depth 13, millions | Peak RSS MB observed |
|---|---:|---:|---:|---:|---:|---:|
| 50 | 9 / 8 / 12 | 0/3 | 11.59 | 11.53–12.00 | 116.24 | 18.7 |
| 75 | 9 / 8 / 12 | 0/3 | 10.55 | 10.54–10.71 | 106.47 | 43.3 |
| 100 | 9 / 8 / 12 | 0/3 | 10.06 | 10.03–11.05 | 101.62 | 67.8 |
| 125 | 9 / 8 / 12 | 0/3 | 10.66 | 9.98–12.86 | 100.23 | 92.4 |
| 150 | 9 / 8 / 12 | 1/3 | 9.94 | 9.83–10.75 | 99.40 | 116.9 |
| 200 | 9 / 8 / 12 | 1/3 | 10.18 | 9.74–11.11 | 98.42 | 166.0 |
| 225 | 9 / 8 / 12 | 1/3 | 11.58 | 9.80–14.32 | 98.26 | 190.4 |
| 250 | 9 / 8 / 13 | 2/3 | 11.87 | 9.87–13.90 | 98.92 | 214.9 |

RAM budgets include reserved headroom for threads, server state, and the allocator. They are ceilings, not forced allocations. Peak RSS is the maximum resident memory observed across this sweep, excludes the browser, and is not the configured budget. Search times include initialization.

## Interpretation

100–150 MB is a reasonable tradeoff for these positions. 150 MB had the fastest median time to endgame depth 13 (9.94 s), but 100 MB was only about 1% slower (10.06 s) while using substantially less RAM. Their timing ranges overlap, so this does not establish 150 MB as a universal winner.

50 MB used the least RAM but needed more repeated work and about 17% more time than 150 MB to finish endgame depth 13. Above 150 MB, median node counts improved only about another 1%, with no consistent timing improvement. Small ordering differences between larger budgets are not strong evidence of an optimum.

250 MB reached endgame depth 13 in two of three ten-second runs. 150, 200, and 225 MB each did so once; smaller budgets did not. This is a real observed threshold difference, but only three samples per budget and overlapping fixed-depth timing ranges limit the confidence of a ranking. All budgets had median opening depth 9 and middlegame depth 8.

This sweep compares RAM ceilings on three positions and two search protocols; it does not establish a best setting for all positions, longer thinking times, or other hardware. No application default was changed.

## Measurements and reproduction

- [Timed results with depth ranges](fine-results.md) · [Raw JSON](fine-results.json)
- [Fixed-depth results with timing ranges](fine-depth13-results.md) · [Raw JSON](fine-depth13-results.json)

```sh
python3 benchmarks/memory/compare.py --time-only --budgets 50 75 100 125 150 200 225 250 --output benchmarks/memory/fine-results.json
python3 benchmarks/memory/compare.py --depth-only --case endgame --depth 13 --budgets 50 75 100 125 150 200 225 250 --output benchmarks/memory/fine-depth13-results.json
```
