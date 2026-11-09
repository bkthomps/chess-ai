# Central-space optimization screens

Two score-preserving calculation changes were tested against the immediately preceding retained engine. Neither is retained in the application. The production engine remains byte-for-byte identical to the source at the start of this investigation.

Baseline is [baseline-engine.rs](baseline-engine.rs), the uncommitted central-space implementation derived from revision `a8408db7a64dc9a13183cebd2a5c85d5c6c6834e`. It matches the archived [original space engine](../central-space/space-engine.rs). SHA-256 values for every tested source are in the raw JSON. Archived candidates are benchmark-only; they are not compiled into the application.

## Candidates and results

1. **Two population counts:** White and Black space regions are disjoint, with twelve squares each. Complementing the Black contribution within its region allows its subtraction to be packed with the White contribution into one population count. Applying this to base/rear space halves four population counts to two. The algebra preserves every score. The measured result is mixed: about 5% faster after e4, but about 3% slower after d4 and 2% slower in the closed-center sample. Other changes are small. Reduced operation counts did not produce a broad search-time improvement.
2. **Symmetry shortcut:** Mirrored pawn bitboards imply mirrored pawn attacks and rear coverage. Matching enemy intrusion masks then guarantee a zero space balance. Skipping the ordinary calculation in that case preserves all scores. Search-time medians changed by roughly -0.3% to +1.2%, with overlapping ranges throughout. This screen does not establish a useful speedup.

| Position | Four → two population counts, time change | Symmetry shortcut, time change |
|---|---:|---:|
| opening | -1.0% | +0.2% |
| after_e4 | -5.1% | +0.6% |
| after_d4 | +2.7% | +1.2% |
| middlegame | -0.3% | -0.0% |
| closed_center | +1.8% | -0.3% |
| endgame | -1.2% | +1.1% |
| tactics | +0.7% | -0.3% |
| pawns | +0.0% | +0.1% |

Positive percentages mean more time. Each candidate has eleven fresh-process serial trials per position, equal completed depths, cold caches, and seeded shuffled variant order. All static child evaluations, fixed-depth scores, best moves, serial nodes, and cache hits agree exactly across variants and repetitions. Existing scalar evaluation/space, symmetry, move-proof, cancellation, mate, history, and terminal tests also passed for each candidate. No evaluation weights, pruning, search targets, thread limits, cache policies, RAM budget, or CPU headroom settings were changed.

Full medians/ranges are in [01-popcounts.md](01-popcounts.md) and [02-symmetry.md](02-symmetry.md); every raw trial, FEN, source hash, compiler/platform, actual cache allocation, and peak RSS where available is in the corresponding JSON files. Small overlapping timing differences are inconclusive. These results are specific to this machine/compiler/workload; they do not show that either technique is universally slower or faster.

## Reproduce

Run without other builds, tests, or performance studies. Only Git, Rust, and Python standard library are needed. Both executables are compiled before timing with `rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`, with LTO enabled as in the release profile. The default total 100,000,000-byte RAM budget and serial limit are equal. Primary timing uses ordinary uninstrumented search; drivers/static diagnostics live only in temporary source copies.

```sh
python3 benchmarks/central-space/compare.py --baseline benchmarks/central-space-optimization/baseline-engine.rs --candidate benchmarks/central-space-optimization/two-popcounts-engine.rs --require-equivalent --threads 1 --modes depth --repetitions 11 --output benchmarks/central-space-optimization/popcounts-follow-up.json

python3 benchmarks/central-space/compare.py --baseline benchmarks/central-space-optimization/baseline-engine.rs --candidate benchmarks/central-space-optimization/symmetry-engine.rs --require-equivalent --threads 1 --modes depth --repetitions 11 --output benchmarks/central-space-optimization/symmetry-follow-up.json
```

The original feature driver retains its original baseline/candidate defaults. Explicit source overrides require a distinct output path; `--require-equivalent` additionally verifies cross-variant static scores, completed-depth scores, and serial search trees. Use separate output paths to preserve the historical screens.

## Interpretation and remaining opportunities

Central space already reuses the material pass, occupancy masks, and pawn attack masks, and performs no allocation or synchronization. The earlier feature comparison showed 40% more serial opening nodes and 31% more after e4; cheaper arithmetic cannot remove that changed search work by itself. Better move ordering or parallel branch balancing could reduce search time while preserving equal-depth scores, but those require separate isolated experiments. Incremental evaluation inputs or a bounded per-thread cache of space inputs are also candidates, not measured improvements here. They introduce state/memory costs and need correctness and budget validation.

No fixed-time or ponder follow-up was pursued because neither serial screen justified retaining a production change. The existing central-space feature studies already record those modes for the retained implementation. These experiments make no Elo claim.
