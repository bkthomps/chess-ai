# Memory budget results

The application now defaults to **100 MB**, selected after these comparisons. The measurements below retain the experimental budgets used for each run.

See also the [finer 50–250 MB sweep](fine-summary.md), which tests the intermediate budgets with the current engine.

Increasing the ceiling to 1 GB did not produce more completed depth than 100 MB in these tests. Larger caches saved some repeated search work, with diminishing returns. Larger budgets remain available for explicit benchmark runs.

Measured on macOS-26.5.1-arm64-arm-64bit, with 8 logical CPUs and six search threads. The original engine is commit `9288315` (roughly 30 MB in use). Every cell is a median of three fresh-process runs. Budgets use decimal MB/GB.

| RAM budget | Depth in 10 seconds: opening / middle / end | Nodes saved at endgame depth 13 | Peak RSS MB in timed tests* |
|---|---:|---:|---:|
| original ~30 MB | 9 / 8 / 12 | — | 32.8 |
| 100 MB | 9 / 8 / 13 | 5.3% | 67.8 |
| 250 MB | 9 / 8 / 13 | 8.7% | 214.9 |
| 500 MB | 9 / 8 / 13 | 8.5% | 267.4 |
| 1 GB | 9 / 8 / 13 | 8.4% | 267.2 |

*Maximum across the three positions of each position’s median peak resident memory. This is actual process usage, not the configured ceiling; the browser is excluded. All fixed-depth scores matched across budgets.

Compared with the original engine, the 1 GB tier searched about 8.4% fewer nodes to finish endgame depth 13. Compared with 100 MB, it searched about 3.4% fewer nodes. Its node count was effectively the same as 250 MB and 500 MB. The extra endgame ply in the timed test was also available with 100 MB; there was no opening or middlegame depth gain.

Elapsed times varied substantially, so these runs do not establish a reliable wall-clock advantage for 1 GB over the smaller new budgets. The deeper endgame target took 10.6–15.6 seconds with 1 GB and 11.5–12.9 seconds with the original engine. The raw runs and ranges are retained rather than treating small differences between medians as precise speedups.

In the separate pondering test (10 seconds on the human position, then e2e4 and a two-second computer search), the original, 100 MB, 500 MB, and 1 GB tiers all had median computer depth 8. The 250 MB tier had median depth 7, with individual runs at both 7 and 8. Its allocated cache was about the same size as the larger tiers and below its ceiling, so that threshold difference does not demonstrate an effect of the capacity limit. These runs show no consistent pondering-depth benefit from the larger ceiling.

The 1 GB tier used at most about 267 MB in the normal timed cases. Growth is driven by collisions rather than the ceiling itself, so unused RAM does not create a large initialization, lookup, or aging cost. Longer or different searches can need more storage; these three positions and thinking times do not establish a universal optimum.

## Memory ceiling check

The tested 1 GB six-thread configuration leaves about 801 MB for cache allocations, with the remaining budget reserved for server state, stacks, scratch, allocator overhead, and retained freed pages. Twenty cycles of filling and dirtying the entire cache, releasing it, and recreating it peaked at **835.2 MB RSS**, below 1 GB. Allocated cache storage reached 800.8 MB.

The extra allocator-retention allowance was added after the search measurements. It only lowers a nonbinding ceiling in those workloads: all recorded working sets were below 270 MB, versus that experimental configuration’s approximately 801 MB cache ceiling. The 100, 250, and 500 MB ceilings are unchanged. Search source hashes are recorded in the JSON files; the repeated full-allocation stress test uses the final allowance.

## Raw results and reproduction

- [Cold timed and fixed-depth results](results.md) · [JSON](results.json)
- [Deeper endgame target and timing ranges](depth13-results.md) · [JSON](depth13-results.json)
- [Pondering results](ponder-results.md) · [JSON](ponder-results.json)
- [Repeated full-cache RAM check](stress.json)

```sh
python3 benchmarks/memory/compare.py
python3 benchmarks/memory/compare.py --case endgame --depth-only --depth 13 --output benchmarks/memory/depth13-results.json
python3 benchmarks/memory/compare.py --case opening --ponder --output benchmarks/memory/ponder-results.json
python3 benchmarks/memory/compare.py --stress-only --repetitions 20 --output benchmarks/memory/stress.json
```

The driver uses only the Python standard library and Rust compiler. It reads the original engine with git show, compiles both versions with identical optimization flags, randomizes budget order, and runs them sequentially. It never checks out or changes the working branch. Each run starts with a cold cache; initialization is included in timing. Peak RSS is recorded where Python exposes per-child resource usage.
