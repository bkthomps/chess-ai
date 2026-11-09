# Piece–pawn relationships

This study isolates new piece–pawn evaluation terms against revision `65810da150f1b7335a1eec98aeebec30412d4ceb`, the immediately preceding pawn-structure evaluator. It changes evaluation explicitly; material, piece-square tables, mobility, central space, pawn structure, phase weights, search/pruning, ordering, target depths, cancellation, cache policy, memory limits, and concurrency remain unchanged.

## Definitions and weights

All weights are original, initial, untuned centipawns for the piece's owner. White/Black totals are subtracted before the existing single phase blend. Promoted pieces use the same terms. The initial board's signed contribution is zero.

| Relationship | Middlegame | Endgame |
|---|---:|---:|
| Supported knight outpost | +20 | +12 |
| Supported bishop outpost | +12 | +8 |
| Knight/bishop immediately behind a friendly pawn | +8 | +4 |
| Each bishop / friendly pawn on the same square color | -2 | -3 |
| Rook on a semi-open file | +12 | +8 |
| Rook on a fully open file | +24 | +12 |

An outpost must be on relative ranks 4–6, attacked by a friendly pawn, and have no enemy pawn strictly ahead on an adjacent file. This excludes current enemy pawn attacks and potential challenges as those pawns advance. Enemy pawns beside or behind the outpost cannot challenge it by forward pushes. File-edge masks prevent wraparound. The estimate ignores blockers, pins, and whether either side can execute the hypothetical pawn moves; it is not a tactical safety guarantee.

The behind-pawn term requires a friendly pawn exactly one square forward of the minor piece on the same file, independent of outpost status. Its modest bonus can stack with an outpost bonus. It does not assert that the pawn actually shelters the piece from all attacks.

Each bishop pays the square-color penalty for every friendly pawn on its square color, including blocked or unblocked pawns. This is a coarse bad-bishop estimate, not an assessment of all useful bishop diagonals; existing mobility still measures available destinations. Multiple bishops of the same color each pay their own penalty. Opposing pawns do not enter this penalty.

A rook gets the semi-open bonus when its file contains enemy pawns but no friendly pawns, and the larger open-file bonus when the file contains no pawns of either side. These two bonuses do not stack. Non-pawn blockers, relative rook rank, and king safety do not alter this term. Pawnless rooks still receive the open-file bonus. Queens receive none of these terms.

## Implementation and verification

Evaluation collects knight masks in its existing board pass and reuses pawn masks, pawn attacks, and diagonal/straight slider masks. Removing queens from the overlapping slider masks identifies bishops and rooks. The relationship calculation uses bit operations and population counts, with no second board scan, per-piece ray search, heap allocation, synchronization, new cache, or retained per-thread state. Pure pawn endings skip it with the existing phase check. Default 100,000,000-byte total server RAM reservations and CPU headroom are unchanged.

Tests compare raw terms with an independent coordinate/piece-list oracle and compare the complete blended evaluation over deterministic playouts. Fixtures cover supported/unsupported outposts, present/future enemy pawn attacks, adjacent pawns that have passed the piece, rank boundaries, file edges, behind-pawn stacking, bishop colors, rook file classification, pawnless rooks, queen exclusion, color/rank mirroring, and the initial position. Existing serial/parallel proof, cancellation, mate/draw/history, clock, pondering, and undo checks also run.

## Reproduction

```sh
python3 benchmarks/piece-pawn-relationships/compare.py
# Preserve historical results when following up:
python3 benchmarks/piece-pawn-relationships/compare.py --repetitions 11 --output benchmarks/piece-pawn-relationships/follow-up.json
python3 benchmarks/piece-pawn-relationships/compare.py --case opening --modes ponder --output benchmarks/piece-pawn-relationships/ponder-follow-up.json
```

Run with no competing builds/tests or CPU-intensive studies. The standard-library-only script loads the pinned baseline and [frozen candidate](relationship-engine.rs). It checks that shared chess, geometry, cache, and ordering source matches baseline. It compiles both temporary executables before timing using `rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat` (LTO enabled as in the application release profile). Only temporary source copies receive benchmark drivers/static diagnostics; the production engine contains no additional instrumentation. Archived engine source is not compiled into the application.

Five repetitions use fresh processes, cold caches, seeded shuffled variant order, and equal memory/thread limits. Both time through equal completed depth and completed depth at 100/500 ms are measured, with serial and automatic threading. Automatic threading preserves the application's headroom policy. Fixed-depth scores must agree between serial and parallel runs within each evaluator; serial scores/moves/nodes must repeat. Feature variants are expected to have different scores/trees. A 60 s guard detects unfinished fixed-depth runs. Timed searches report the last fully completed iteration, not unfinished work.

[Raw results](results.json) record baseline/source hashes, compiler flags/platform, FENs/depths, repetitions, actual threads, total RAM budget, actual cache allocation, peak RSS where available, scores, moves, nodes/hits, elapsed times, and static root-child diagnostics. [The measurement table](results.md) gives medians and ranges. Warm-cache tests use a 500 ms pondering warmup, e2e4, then a measured 2 s reply; each trial still starts a fresh process.

## Results

Five repetitions yielded 480 cold-cache measurements. Automatic threading selected six threads. Median time through the same completed depth is below; positive changes mean slower. Ranges appear in [the full table](results.md). Small differences with overlapping ranges are inconclusive, so the table does not establish small general speed improvements.

| Position / depth | Serial: before → relationships, ms | Six threads: before → relationships, ms |
|---|---:|---:|
| Initial / 8 | 855.00 → 1138.78 (+33.2%) | 284.58 → 561.10 (+97.2%) |
| After e4 / 7 | 383.63 → 481.85 (+25.6%) | 213.68 → 243.92 (+14.1%) |
| After d4 / 7 | 212.81 → 208.91 (-1.8%) | 61.98 → 62.13 (+0.2%) |
| Middlegame / 6 | 509.09 → 509.99 (+0.2%) | 241.27 → 238.28 (-1.2%) |
| Closed center / 6 | 202.46 → 202.16 (-0.1%) | 71.38 → 69.90 (-2.1%) |
| Rook ending / 10 | 825.18 → 788.72 (-4.4%) | 316.73 → 284.51 (-10.2%) |
| Promotion tactics / 6 | 206.11 → 206.51 (+0.2%) | 59.75 → 60.60 (+1.4%) |
| Pawn ending / 10 | 59.70 → 59.27 (-0.7%) | 24.45 → 23.48 (-3.9%) |

The pawn-ending FEN is labeled `relationships` in the raw case list; it remains a pure pawn-ending control. Serial scores/moves/nodes repeat, and within each evaluator all fixed-depth serial/parallel scores agree. The pawn-ending serial node count and score match baseline exactly because this term is skipped at zero phase. Its small timing changes are noise, illustrating the limits of timing-only comparisons.

The initial depth-8 best move changed from Nc3 to d4, both scored 0. This does not prove d4 is better: these are search choices from equal-valued alternatives under different evaluations. All seven other sampled fixed-depth best moves stayed the same; scores changed after e4, in the middlegame/closed center, and in the promotion position. The initial static scores for Nc3/e4/d4 changed from 74/62/60 to 66/62/60. Nc3 loses the behind-pawn bonus it had on b1; neither central double push loses its bishops' square-color terms. At shorter completed depths the engine can still prefer knights.

Opening serial nodes increased from 2,177,558 to 2,952,863 (+35.6%). After e4 they increased from 986,293 to 1,164,740 (+18.1%). The opening regression therefore includes changed search work; parallel scheduling also changes how much work runs before branches finish. These are total search costs, not isolated per-evaluation overhead. The rook ending searched fewer serial nodes (2,537,655 → 2,338,810).

At equal time, the initial six-thread 500 ms search dropped from completed depth 8 to 7 in all trials. The serial 100 ms rook ending increased from depth 7 to 8 in all trials. Other tested allowances generally reached the same depth; one parallel pawn-ending trial crossed an iteration boundary. Peak measured process RSS in the cold study was 32.03 MB for these workloads. The default reservations remain unchanged; this observation is not a general maximum-memory proof.

The separate [warm-cache table](ponder.md) and [20 raw trials](ponder.json) show reply depth 8 serially in all trials for both variants after e2e4, with the new choice d5 versus Nc6. With six threads the new evaluator completed depth 9 in all five trials versus depth 8 before. Warm-study peak RSS was 66.09 MB. Warm-cache outcomes are position-dependent and do not cancel the cold opening regression or establish a general speed/strength gain.

## Verification status

The candidate passed `cargo fmt -- --check`, `cargo test --offline` (105 passed; two benchmark tests ignored), and `cargo clippy --all-targets --offline -- -D warnings`. `cargo build --release --offline`, `python3 tests/http_smoke.py`, and `node tests/ui_smoke.cjs` also passed, including click/drag moves, cancellation, pondering, and stale-result isolation. Frozen production source and measurement hashes, script syntax, documentation links, and `git diff --check` are verified.

## Limitations

Changed evaluations change search work and parallel branch balance; search timings combine this behavior with calculation overhead. Small overlapping timing ranges are inconclusive. Equal-time iteration boundaries can vary with scheduling. These tests do not establish an Elo gain or tune the weights. Rook-file, outpost, and bishop-color heuristics remain approximations and require broader strength testing. No king shelter/storm, rook-behind-passer, candidate-outpost, or tactical pawn-threat terms are added here.
