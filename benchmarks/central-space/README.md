# Central space evaluation

This study isolates adding central space to the optimized mobility engine at baseline revision `a8408db7a64dc9a13183cebd2a5c85d5c6c6834e`. It intentionally changes evaluation. Different scores, trees, and best moves between variants are expected; material, piece-square formulas, mobility weights, king endpoints, phase weights, pruning, ordering, depth targets, cache rules, time management, and concurrency policy retain their existing semantics.

## Definition

The central region is files c–f and relative ranks 2–4: c2–f4 for White, c5–f7 for Black. A square counts if it is empty or occupied by a friendly non-pawn piece, and is not attacked by an enemy pawn. Friendly pawns and every enemy occupant exclude it. This estimates pawn-safe room, not full tactical safety or legal destinations. Enemy bishops, knights, rooks, and queens do not supply additional attack exclusions.

Each qualifying square earns four middlegame centipawns. It earns another four if a friendly pawn is one, two, or three ranks ahead on the same file. Multiple pawns whose rear regions overlap do not multiply the second bonus. A pawn can also restrict the opponent's space through its attacks or occupation. The signed score is White's territory bonus minus Black's.

The endgame component is zero. Space uses the existing material-phase blend (24 initially, knight/bishop 1, rook 2, queen 4, capped at 24 for promotions). Its signed correction is blended once alongside king and mobility terms, preserving integer-rounding color symmetry. The term is entirely disabled when no pawns remain; at phase zero the actual evaluation has no space contribution. Pawn-only positions keep their mobility/space setup fast path.

The maximum raw bonus is 96 cp per side (twelve squares each counted at most twice × four), before phase blending. The four-centipawn weight is an original, conservative initial choice, not a copied Stockfish table or a tuned strength claim. Phase blending rather than a new hard material threshold controls its decline.

## Implementation and verification

Occupancy and pawn masks are already collected by the evaluator's existing board pass. Pawn attack masks are now computed once and supplied to both space and mobility; the mobility definition is unchanged. Space needs only shifts, masks, and four population counts. No extra board scan, heap allocation, retained cache, per-thread table, lock, atomic, dependency, asset, or platform-specific path is introduced. The default total RAM budget remains 100,000,000 bytes and the CPU headroom policy remains in force.

Tests check a symmetric initial position, central pawn pushes versus knight development, enemy pawn control and occupancy, friendly non-pawn occupancy, region edges, three-rank rear coverage, overlapping pawns, mirroring colors/ranks, and complete removal of the term in a pawn ending. A separate scalar coordinate oracle uses no production masks, shifts, or population counts. It is checked over the existing deterministic playouts, both for raw space and full blended evaluation. Existing proven-move, serial/parallel, mate, draw, terminal, cancellation, history, pondering, clock, and undo checks cover the updated evaluation.

## Method and reproduction

Run without competing CPU-intensive builds, tests, or studies:

```sh
# Five fresh-process trials, fixed depths and 100/500 ms budgets, one and automatic threads
python3 benchmarks/central-space/compare.py

# Follow-up measurements, preserving historical output
python3 benchmarks/central-space/compare.py --repetitions 11 --output benchmarks/central-space/follow-up.json

# Warm-cache ponder simulation: 500 ms initial search, e2e4, two-second reply
python3 benchmarks/central-space/compare.py --case opening --modes ponder --output benchmarks/central-space/ponder-follow-up.json
```

The script loads the baseline from its pinned revision and the candidate from archived [space-engine.rs](space-engine.rs), whose hash matches the measured application source. The archive is benchmark-only; it is not compiled into the application. `--candidate src/engine.rs` explicitly selects an alternative source and requires a distinct output path. Shared board/geometry/cache/ordering sources must match baseline. Only Git, Rust, and Python's standard library are required.

Both temporary executables finish compiling before measurements. Flags are `rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`; LTO is enabled, as in the application release profile. Drivers/static diagnostics are appended only to temporary source copies. Production search contains no added benchmark instrumentation.

Trials use fresh processes, cold caches, seeded shuffled variant order, equal 100 MB total budgets, and equal thread limits (one and automatic). Fixed-depth timing includes initialization and every iteration through the target, with a 60-second failure guard. Time-limited results retain only the last completed depth/score. Within each evaluator the script checks serial/parallel equal-depth score agreement and repeatable serial score/move/nodes. Cross-variant score equality is not expected for an evaluation change.

[results.json](results.json) retains baseline revision, source hashes, compiler/platform/flags, eight FENs, repetitions, thread configuration, actual cache allocation, peak RSS where supported, scores, moves, nodes, hits, and times. [results.md](results.md) gives medians and ranges. Static evaluations of all root alternatives are recorded separately, outside timing. The positions cover initial play, replies to e4/d4, a developed middlegame, a closed French pawn center, a rook ending, promotion tactics, and a pawn ending.

## Results

The primary comparison contains **480 measurements**: eight positions, five fresh-process repetitions, one/six threads, fixed depths, and 100/500 ms search allowances. Within each evaluator, equal-depth serial/parallel scores agree and serial repetitions repeat exactly the same move/score/node count. Scores between evaluators intentionally differ.

In the starting position, the raw static score of e4 increases **52 → 68 cp**, d4 **50 → 66 cp**, and Nc3 stays **74 cp**. The +16 for each central double push combines extra room behind the pawn with restricted enemy central space. These are static scores, not completed search evaluations.

| Position | Depth | Score / move before | Score / move with space | Serial median ms before → with space | Six-thread median ms before → with space |
|---|---:|---|---|---|---|
| opening | 8 | +0 / b1c3 | +0 / b1c3 | 564.60 → 836.87 | 184.55 → 290.06 |
| after_e4 | 7 | +11 / b8c6 | +0 / d7d5 | 259.51 → 410.72 | 87.29 → 231.46 |
| after_d4 | 7 | +0 / g8f6 | +0 / g8f6 | 197.03 → 192.71 | 61.46 → 63.84 |
| middlegame | 6 | -7 / d5e6 | +4 / d5e6 | 524.67 → 509.80 | 259.70 → 247.73 |
| closed_center | 6 | +9 / g1f3 | +21 / g1f3 | 208.76 → 209.63 | 77.05 → 80.30 |
| endgame | 10 | +20 / b4f4 | +20 / b4f4 | 690.06 → 598.39 | 267.76 → 235.64 |
| tactics | 6 | +481 / d7c8q | +477 / d7c8q | 213.98 → 214.35 | 62.10 → 61.48 |
| pawns | 10 | +28 / g3f3 | +28 / g3f3 | 46.06 → 45.73 | 15.94 → 18.90 |

The initial move remains **Nc3**. After e4, the equal-depth reply changes **Nc6 → d5** (the Scandinavian Defense). Other equal-depth best moves in this suite remain the same. The closed French center changes score +9 → +21 cp while keeping Nf3. Scores are raw centipawns from the side to move.

**Performance regressions are substantial in the opening.** Time to initial depth eight rises about 48% serially and 57% with six threads; the serial node count grows 40%. After e4, depth-seven time rises about 58% serially and 165% with six threads, with 31% more serial nodes. New evaluations alter pruning, the leading branch, and parallel work distribution; these timings are total search costs, not a pure per-evaluation microbenchmark.

The rook ending uses 17% fewer serial nodes and takes about 13% less serial time (12% less six-thread time), despite the root space term fading strongly there. The changed term can still affect searched continuations and cache bounds. The middlegame median is slightly lower and after d4 results are mixed. Small differences in the closed-center, tactical, and pawn-ending controls have overlapping timing ranges and do not establish general effects. All ranges and nodes are retained in results.md/results.json.

These measurements show a new positional preference, not stronger play. The opening regressions and changing reply require further tuning/strength tests before claiming an improvement. The initial four-centipawn weight was not adjusted to force a pawn opening or to improve these benchmark timings.

Maximum observed search-process RSS in this short primary study was 28.0 MB, excluding browser/server. Actual cache allocation is recorded per trial; this does not fill the cache or establish a worst-case memory bound.

### Timed search and warm-cache behavior

Most completed-depth settings are unchanged, but after e4 with six threads the 500 ms allowance falls from depth eight to seven in all five trials. At 100 ms, baseline reaches seven in four trials and six in one, while space reaches six throughout. After d4 at 500 ms, baseline reaches eight once and seven four times; space reaches seven throughout. In the other positions, any depth differences are small timing-boundary variations; full per-trial data is retained.

Twenty additional [warm-cache trials](ponder.json) use a 500 ms bounded ponder of the initial position, then e2e4 and a two-second reply with the retained cache. Both versions complete ponder depth seven serially and eight with six threads, then reply depth eight throughout. At that deeper completed depth, both choose Nc6: the score changes -31 → -48 cp. Thus the d5 choice at fixed depth seven is not universal across depths or allowances. Peak search-process RSS in these trials is 65.7 MB before and 61.7 MB with space, excluding browser/server. Cache utilization depends on the searched tree within the same configured limits.

## Verification status

Formatting, offline Rust tests, Clippy with warnings denied, the offline release build, HTTP smoke checks, and the UI smoke harness pass. In particular, the drag/cancellation regression tests remain passing. Source hashes in the recorded study match the archived and application engine sources.

Score-preserving arithmetic and symmetry shortcuts were also [screened separately](../central-space-optimization/README.md). Neither showed a consistent overall speedup, so the original calculation remains in use.

## Limitations

Different evaluations alter search trees, so total timing differences combine the added evaluation cost with different pruning and move ordering outcomes. Short-time depth changes are sensitive to scheduling and iteration boundaries. Equal-valued moves can differ and parallel node counts vary. These positions and measurements do not establish Elo or comprehensively test pawn structures, king safety, or positional understanding. Initial weights require broader strength testing and tuning.
