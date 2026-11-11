# Deferred move ordering: rejected screens

Neither proposal is retained. Both compare against revision `162cd89`, using identical search, evaluation, total 100,000,000-byte RAM budget, and thread limits. The candidate changes when sorting happens, preserving priorities and generated-order ties.

- `selection.patch`: select and stably extract the highest-priority remaining move on every request. The serial screen was stopped after seven pairs each for opening, after e4, and after d4, and two pairs for middlegame. Median equal-depth times regressed by 6.0%, 8.4%, and 37.9% in those opening cases. The partial measurements are in `screen.json`; this is not a completed suite.
- `candidate.patch`: select the first move, then insertion-sort the remainder only when a second move is needed. Seven serial pairs per position completed. Results were mixed, with broad overlapping timing ranges and regressions after d4 and in the rook ending. See [first-screen.md](first-screen.md) and [raw trials](first-screen.json). This did not establish a reliable broad improvement, so no fixed-time or ponder follow-up was pursued.

The completed screen verifies identical fixed-depth scores, best moves, nodes, and cache hits between variants and repetitions. The initial selection candidate also passed the existing test suite before timing. Neither screen changes evaluation or pruning. No depth or playing-strength gain is claimed.

Reproduce into a new output path to preserve these records:

```sh
python3 benchmarks/lazy-ordering/compare.py --patch benchmarks/lazy-ordering/selection.patch --threads 1 --modes depth --repetitions 7 --output benchmarks/lazy-ordering/selection-follow-up.json
python3 benchmarks/lazy-ordering/compare.py --threads 1 --modes depth --repetitions 7 --output benchmarks/lazy-ordering/first-follow-up.json
```

The driver reconstructs the frozen baseline with Git and applies the archived patch in temporary copies. Rust and Python standard libraries only. Both binaries compile before timing with optimization level 3, fat LTO, and one codegen unit, matching the application release profile. Every trial is a fresh process with cold caches; variant order is shuffled with a recorded seed in the script. Raw JSON records FENs, compiler/platform, source hashes, thread counts, actual cache allocations, and peak RSS where available. No other task-run builds, tests, or benchmark studies ran during timing. Large time ranges indicate environmental noise; small differences are inconclusive.
