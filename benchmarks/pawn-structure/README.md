# Pawn structure evaluation

This study isolates adding pawn-structure scoring to revision `d33bf7974101d2190ea07e117e38e658fddb3122`, which already includes optimized mobility, central space, and gradual phase blending. Evaluation changes intentionally. Different scores, search trees, and moves are expected. Material values, piece-square tables, mobility/space weights, king endpoints, phase weights, pruning, ordering, target depths, time allocation, cache policy, memory budget, and concurrency policy retain their existing semantics.

## Definitions and weights

All terms use the two pawn bitboards only. Scores below are centipawns for the pawn's owner, blended with the existing material phase (24 initially; knights/bishops 1, rooks 2, queens 4; capped at 24 for promotions). White and Black raw totals are subtracted before blending with other phase-dependent terms, preserving color symmetry and the existing single rounding step.

| Term | Definition | Middlegame | Endgame |
|---|---|---:|---:|
| Isolated | No friendly pawn anywhere on either adjacent file | -12 per pawn | -18 per pawn |
| Doubled | Excess pawns on a file: a file with n pawns incurs n-1 penalties | -10 per excess pawn | -16 per excess pawn |
| Backward | Non-isolated pawn whose one-step stop is pawn-empty and enemy-pawn-attacked, with no possible pawn support from an adjacent file at the same rank or behind | -8 per pawn | -12 per pawn |
| Connected | Supported by a friendly pawn or beside one on the same rank | +6 per pawn | +10 per pawn |

A pawn satisfying both connection conditions earns one bonus. Isolated pawns do not also receive the backward penalty. Isolated/doubled penalties can stack. A pawn-blocked stop is not classified as backward by this term. Support potential ignores the intervening moves and non-pawn blockers; backward detection is a static heuristic, not proof of permanent weakness or move legality.

A passed pawn has no enemy pawn strictly ahead on its own file or either adjacent file. Only the frontmost friendly pawn on a file receives this bonus, preventing rear doubled pawns from independently earning it. Enemy pawns alongside the pawn on the same rank do not disqualify it. Passed-pawn bonuses are:

| Relative rank | Middlegame | Endgame |
|---|---:|---:|
| 2 | 0 | 5 |
| 3 | 4 | 10 |
| 4 | 10 | 20 |
| 5 | 20 | 40 |
| 6 | 35 | 70 |
| 7 | 60 | 120 |

Relative ranks are mirrored for Black. Back-rank pawn entries are zero defensively; legal pawns promote on reaching their final rank. These original initial weights are untuned and are not copied Stockfish tables. This implementation does not add candidate-passer scoring, king-distance/path safety, pawn shelter, pawn storms, or non-pawn blockade adjustments. Search resolves tactical consequences; a passed-pawn bonus is not a promotion guarantee.

## Implementation and correctness

The existing evaluation pass already gathers both pawn masks. Shared pawn attacks now feed mobility, space, and pawn structure, including in pawn endings. Vertical fills and adjacent-file masks identify weaknesses, connections, and passers without another board scan, allocation, lock, atomic, retained per-thread state, external asset, or dependency. Passed-pawn rank bonuses iterate only the set passer bits. The 100,000,000-byte total RAM budget and portable CPU headroom policy remain unchanged.

The retained application source is frozen as [uncached-engine.rs](uncached-engine.rs) for reproduction; it is a benchmark-only archive, not compiled into the application. [pawn-engine.rs](pawn-engine.rs) is an archived rejected one-entry-cache experiment, described below. Source hashes identify both precisely.

Tests use a separate coordinate/piece-list oracle with no production masks/fills/popcounts. Fixtures cover isolated/doubled pawns, phalanxes and support, non-stacking connection bonuses, backward stops, blocked advances, isolation/backward overlap, frontmost passers, adjacent enemy pawns, and color/rank mirroring. Raw pawn terms and complete blended evaluation are checked over deterministic playouts including captures and promotion. Existing independent move-proof, serial/parallel, mate normalization, terminal/draw, cancellation, history, pondering, clock, and undo tests remain applicable.

## Method and reproduction

Run without competing builds, tests, or CPU-intensive studies. Only Rust, Git, and Python's standard library are needed.

```sh
# Main feature comparison: five fresh trials, fixed depths and 100/500 ms allowances
python3 benchmarks/pawn-structure/compare.py

# Follow-up: keep prior measurements intact
python3 benchmarks/pawn-structure/compare.py --repetitions 11 --output benchmarks/pawn-structure/follow-up.json

# Warm-cache pondering simulation: 500 ms warmup, e2e4, then a 2 s reply
python3 benchmarks/pawn-structure/compare.py --case opening --modes ponder --output benchmarks/pawn-structure/ponder-follow-up.json
```

The baseline loads from the pinned revision and candidate from the frozen retained source. Shared chess/geometry/cache/ordering files must match baseline. Both temporary executables compile before timing with `rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`, enabling LTO as in the application release profile. Static-score diagnostics and drivers are appended only to temporary source copies; normal search has no extra benchmark instrumentation.

Each trial uses a fresh process, cold cache, seeded shuffled variant order, the same total 100 MB budget, and equal one-thread/automatic-thread limits. Fixed-depth timing includes initialization and all completed iterations through the target, with a 60-second failure guard. Timed search reports only its last fully completed iteration. Within each evaluator, serial and parallel equal-depth scores must agree, and serial repetitions must repeat scores/moves/nodes. Cross-variant scores are expected to differ for the feature comparison.

[results.json](results.json) records all eight FENs, baseline/source hashes, compiler/platform/flags, repetitions, actual thread counts, RAM budget, actual cache allocation, peak process RSS where available, elapsed time, completed depth, score, move, nodes, and hits. [results.md](results.md) gives medians and ranges. Static scores for legal root alternatives are recorded separately outside timing. These are search-behavior/performance measurements, not an Elo tournament.

## Results

Five trials per variant/configuration produced 480 cold-cache measurements. Median time through the same completed depth is shown below; percentages describe elapsed-time changes, with positive values slower. Automatic threading selected six threads on this machine. See the raw table for ranges: several apparent small gains overlap timing noise, especially the initial-position serial result.

| Position / depth | Serial: before → pawn structure, ms | Six threads: before → pawn structure, ms |
|---|---:|---:|
| Initial / 8 | 999.74 → 907.52 (-9.2%) | 314.14 → 321.88 (+2.5%) |
| After e4 / 7 | 430.53 → 399.47 (-7.2%) | 242.83 → 233.99 (-3.6%) |
| After d4 / 7 | 200.69 → 219.75 (+9.5%) | 69.10 → 67.70 (-2.0%) |
| Middlegame / 6 | 532.94 → 527.40 (-1.0%) | 269.16 → 257.72 (-4.2%) |
| Closed center / 6 | 223.11 → 213.83 (-4.2%) | 83.63 → 75.96 (-9.2%) |
| Rook ending / 10 | 677.78 → 917.34 (+35.3%) | 251.64 → 367.28 (+46.0%) |
| Promotion tactics / 6 | 249.69 → 244.63 (-2.0%) | 82.27 → 77.73 (-5.5%) |
| Pawn ending / 10 | 43.21 → 63.82 (+47.7%) | 18.03 → 27.15 (+50.6%) |

All eight sampled fixed-depth best moves stayed the same. Scores changed in the middlegame, closed center, rook ending, tactics, and pawn ending. Within each evaluator, serial/parallel fixed-depth scores agreed. The rook ending searched about 47% more serial nodes and the pawn ending about 46% more; their timing regressions therefore include substantially more search work, rather than representing the isolated cost of calculating pawn terms.

Equal-time tests also expose the ending cost. At 100 ms, the rook ending dropped from depth 8 to 7 serially and 9 to 8 with six threads; the pawn ending dropped from 11 to 10 and 12 to 11 respectively. At 500 ms, the serial pawn ending completed depth 12 with pawn structure versus depth 13 in four of five baseline trials. Conversely, after d4 the six-thread 500 ms search completed depth 8 in all new trials versus depth 7 before. Other configurations generally completed the same depth, with occasional boundary variation recorded in the raw data.

This feature does not resolve the initial knight preference: the initial depth-8 search still chooses Nc3. Static child scores for Nc3/e4/d4 were 74/68/66 before and 74/62/60 after. Moving a central pawn alone breaks its same-rank connections, costing 6 middlegame centipawns with these initial weights. This is a concrete tuning limitation; the new terms should not be presented as proven opening or strength improvements.

The separate [warm-cache study](ponder.md), with [20 raw trials](ponder.json), completed reply depth 8 in all six-thread trials for both variants. Serially it completed depth 8 in all five new trials and four baseline trials (one depth 7). This small sample does not establish a pondering speed improvement. Maximum measured process RSS was 27.90 MB in the cold study and 61.15 MB in the warm study, below the default total budget for these workloads; the existing reservation policy remains in force.

## Rejected cache experiment

Before the main feature study, a single-entry cache per search thread was tested separately against the otherwise identical uncached pawn evaluator. It compared both full pawn bitboards, stored only the raw middlegame/endgame pawn terms, and charged its 24-byte entry to the existing 512 KiB scratch limit. It did not cache whole evaluations or transposition bounds. Tests covered both pawn colors, reused shapes with different non-pawn material/side to move, captures, promotion, and empty pawn sets.

Five fresh-process serial equal-depth trials per position showed small mixed timing differences, approximately -1% to +1%, rather than a reliable gain. All static evaluations and fixed-depth scores, moves, nodes, and cache hits agreed. The cache was **not retained**; the screen does not establish that larger pawn hash tables are ineffective. [cache-screen.json](cache-screen.json) and [cache-screen.md](cache-screen.md) preserve the raw evidence. Reproduce using the archived cached candidate explicitly:

```sh
python3 benchmarks/pawn-structure/compare.py --baseline benchmarks/pawn-structure/uncached-engine.rs --candidate benchmarks/pawn-structure/pawn-engine.rs --require-equivalent --threads 1 --modes depth --output benchmarks/pawn-structure/cache-follow-up.json
```

No fixed-time/warm-cache follow-up was pursued for the rejected optimization. The main retained-feature comparison measures both modes separately. Explicit source overrides require a distinct output path to preserve prior results.

## Verification

The retained uncached application passed `cargo fmt -- --check`, `cargo test --offline` (104 passed, two benchmark tests ignored), `cargo clippy --all-targets --offline -- -D warnings`, `cargo build --release --offline`, `python3 tests/http_smoke.py`, and `node tests/ui_smoke.cjs`. The UI harness passed drag/click moves and cancellation. Source hashes match the retained/archive variants used for their recorded measurements. The benchmark script parses and `git diff --check` passes.

## Limitations

Different evaluations change pruning outcomes, branch sizes, and parallel load balance. Total search timing combines added calculation cost and different search work; it is not a per-evaluation microbenchmark. Small overlapping timing ranges are inconclusive, and short time budgets are sensitive to iteration boundaries and scheduling. Faster searches, changed scores, fewer nodes, or extra completed plies do not establish stronger play. Pawn structures and initial weights require broader strength testing/tuning; this suite does not comprehensively cover pawn races, blockades, fortresses, or king safety.
