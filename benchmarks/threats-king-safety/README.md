# Threats and king safety

This study adds threat and king-safety terms to revision `3868af714fc9c610ad85093d8058a1462cf32c0a`, the immediately preceding relationship evaluator. This is an explicit evaluation change. Material, existing piece-square endpoints, bishop-pair bonus, mobility/space/pawn/relationship weights, phase weights, pruning, ordering, target depths, time allocation, cache semantics, memory budget, and concurrency policy are preserved.

## Threat definitions and weights

Scores are original untuned centipawns for the attacking side, written middlegame/endgame. Each attacked non-king victim receives the **largest applicable** bonus in each phase, once. Attacker counts do not multiply it and categories do not stack.

| Victim | Attacked and undefended | Attacked by a pawn | Attacked by a cheaper non-pawn class |
|---|---:|---:|---:|
| Pawn | 6 / 10 | 0 / 0 | 0 / 0 |
| Knight or bishop | 16 / 20 | 24 / 32 | 0 / 0 |
| Rook | 24 / 30 | 40 / 48 | 20 / 24 |
| Queen | 40 / 50 | 60 / 72 | 32 / 40 |

The cheaper non-pawn classes are knight/bishop attacks on rooks/queens and rook attacks on queens. Knights and bishops are treated as one class here despite their distinct material values. The pawn/cheaper-class bonuses can apply to defended victims. Undefended means no friendly pawn, king, or pin-restricted non-pawn attack/defense reaches the target. Pure pawn endings receive threat terms too. Kings are never threat victims; check/mate remains search's responsibility.

This is positional guidance, not static exchange evaluation, a proof that an exchange wins material, a fork detector, or a legal capture generator. It does not simulate counterchecks, recapture sequences, en passant, pawn pushes/promotions, or move order. Both sides' threats contribute symmetrically regardless of whose turn it is. Pinned pawns still supply geometric attacks; king defenses are adjacent squares without testing the legality of a recapture.

## King safety

The zone is the king's square plus its valid adjacent squares, clipped at file/rank edges. Pin-restricted non-pawn attacks are shared with mobility; pawn/king attacks are geometric. Sliding attack maps stop at the first blocker and include that blocker, so friendly pieces can be represented as defended squares. A pinned knight contributes no non-pawn attacks; a pinned slider is restricted to its king/pinner line. Existing mobility still excludes friendly destinations, enemy kings, and enemy pawn-controlled squares exactly as before.

For each defending king, raw danger adds:

- **Zone pressure:** pawn-attacked zone squares contribute one unit each, with overlapping pawn attacks counted once. Each attacking knight/bishop contributes two units per zone square, rook three, queen five, capped at four touched squares per piece. Kings do not add pressure units. With `u` units, danger is `2*u + u*u/16` (integer division). The quadratic part increases coordinated pressure without legal checking-move searches.
- **Weak zone squares:** four danger points per enemy-attacked zone square lacking friendly **non-king** defense. King adjacency remains valid defense for hanging-piece classification but cannot by itself make an attacked escape square safe.
- **Pawn shield:** for kings on relative ranks 1–3 only, examine the king file and valid adjacent files. The nearest friendly pawn one/two/three ranks ahead costs 0/4/8 danger points respectively; no pawn within that distance costs 12. Pawns beside/behind the king do not provide a forward shield.
- **Pawn storm:** on those same files, the nearest enemy pawn one/two/three ranks ahead costs 12/8/4 points. This applies at any king rank. It does not model pawn-blocking sequences, breaks, or whether the storm pawn can advance.
- **Exposed files:** if the enemy has a rook or queen, each neighboring/king file without a friendly pawn costs six points, plus four more when it contains no enemy pawn either. File presence considers all ranks; non-pawn blockers do not change this term.

Cap the combined raw danger at 300 per king, then multiply by opposing attacking material divided by 12. Attacking material uses knight/bishop=1, rook=2, queen=4, capped at 12 for promotions. A bare opposing king/pawns therefore cause no king-safety penalty by themselves. Subtract each side's positive penalty, then apply the existing global phase blend with endgame weight zero for safety. The two scaling steps intentionally reduce safety's influence as attacking material disappears; existing king activity scores remain independent.

All threat and safety totals join the existing single signed phase blend, preserving its rounding and mirrored-color symmetry. No additional tempo bonus, safe-check scoring, king-move search, castling-right forecast, or attacker-material adjustment to other evaluation terms is introduced.

## Implementation and verification

The existing evaluation board pass supplies occupied/pawn/knight/slider masks and king squares. Each non-pawn's raw attacks are computed once in the existing mobility loop and reused for mobility, target attacks/defenses, and zone pressure. Local union/minor/rook/non-king attack maps use ordinary integers. King neighborhoods are compiled immutable masks. Only attacked victims are iterated for threats; king pawn scans cover at most three files and three forward ranks. There is no new dependency, heap allocation, lock, retained per-thread state, or cache. Default 100,000,000-byte total RAM reservations and portable CPU headroom remain unchanged.

A separate coordinate/path-walking oracle supplies raw attacks, including friendly blockers, absolute pins, pawns, and kings, without production attack geometry. Independent square-list threat/safety calculations use literal reference weights and coordinate neighborhoods. Deterministic playouts check raw terms, individual non-pawn attack maps, complete blended evaluations, mirror symmetry, and unchanged mobility counts. Focused fixtures cover defended/undefended targets, pawn/cheaper-piece attacks, one target with multiple attackers, blockers, pinned knights, king defenses, shields/storms, file edges, bare enemy material, and capped promoted-material danger. Existing serial/parallel proof, terminal/mate/draw, history restoration, cancellation, pondering, clock, and undo tests continue to apply.

## Method and reproduction

```sh
python3 benchmarks/threats-king-safety/compare.py
# Keep historical data intact when repeating:
python3 benchmarks/threats-king-safety/compare.py --repetitions 11 --output benchmarks/threats-king-safety/follow-up.json
python3 benchmarks/threats-king-safety/compare.py --case opening --modes ponder --output benchmarks/threats-king-safety/ponder-follow-up.json
```

Run without competing builds, tests, or performance studies. The standard-library-only script loads the pinned baseline and [frozen candidate](safety-engine.rs); this archive is not compiled into the application. Shared chess/tables/cache/ordering files must match baseline. Both executables compile before measurements with `rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`, enabling LTO as in the release profile. Drivers and static diagnostics are appended only to temporary source copies. Normal search has no additional measurement instrumentation.

Five fresh-process trials use cold caches, seeded shuffled variant order, the same total 100 MB budget, and equal serial/automatic threading limits. Nine FENs cover the prior eight cases plus kingside attack pressure. Equal completed depths and completed depths at 100/500 ms are recorded; timed runs return the last completed iteration. Fixed-depth scores must agree between serial/parallel configurations within each evaluator, and serial repetitions must repeat scores/moves/nodes. Scores can differ between evaluators by design. A 60 s fixed-depth guard detects unfinished work. Warm-cache tests use 500 ms pondering, e2e4, then a measured 2 s retained-cache reply in each fresh process.

[Raw results](results.json) contain compiler/platform/flags, source/baseline hashes, FENs, repetitions, actual threads, memory budget, actual cache allocation, peak RSS where available, nodes/hits, scores/moves, completed depth, elapsed time, and static legal-root-child scores. [The table](results.md) reports medians and ranges. Search times combine changed search trees with extra evaluation calculation; they are not isolated per-evaluation microbenchmarks. Small overlapping timing differences are inconclusive. These measurements do not establish an Elo gain or tune the weights.

## Results

Five repetitions produced 540 cold-cache measurements. Automatic threading selected six threads on this machine. Median time through the same completed depth is below, with positive changes slower. Full ranges appear in [results.md](results.md); small overlapping differences are inconclusive.

| Position / depth | Serial: before → new, ms | Six threads: before → new, ms |
|---|---:|---:|
| Initial / 8 | 1121.00 → 797.04 (-28.9%) | 528.03 → 292.57 (-44.6%) |
| After e4 / 7 | 475.93 → 469.80 (-1.3%) | 238.33 → 287.63 (+20.7%) |
| After d4 / 7 | 206.46 → 245.28 (+18.8%) | 56.36 → 77.96 (+38.3%) |
| Middlegame / 6 | 504.43 → 609.70 (+20.9%) | 236.50 → 297.16 (+25.7%) |
| Closed center / 6 | 200.33 → 247.02 (+23.3%) | 70.20 → 117.73 (+67.7%) |
| Rook ending / 10 | 785.58 → 1055.99 (+34.4%) | 286.85 → 494.92 (+72.5%) |
| Promotion tactics / 6 | 206.19 → 232.19 (+12.6%) | 60.79 → 66.96 (+10.2%) |
| Pawn ending / 10 | 59.37 → 71.85 (+21.0%) | 23.85 → 34.74 (+45.7%) |
| King attack / 6 | 510.50 → 899.47 (+76.2%) | 203.06 → 326.41 (+60.7%) |

Within each evaluator all fixed-depth serial/parallel scores agreed and serial repetitions repeated scores/moves/nodes. Eight of nine sampled best moves stayed the same. The initial depth-8 choice changed from d4 to Nc3, both scored 0; scores changed in six of the other positions. This is not evidence that Nc3 is better.

The initial static Nc3/e4/d4 scores changed from 66/62/60 to 66/54/52. A central double pawn push moves a shield pawn from distance one to three in front of the current e1 king, costing eight middlegame centipawns. This initial shelter implementation evaluates the current king square and does not forecast castling. It therefore increases the initial static knight preference, a concrete limitation to consider in strength testing and tuning. It should not be presented as resolving the original opening preference.

Changed search work explains part of both gains and regressions: initial serial nodes fell 2,952,863 → 1,824,739; rook-ending nodes rose 2,338,810 → 3,092,729; king-attack nodes rose 843,350 → 1,235,759. Even the pawn ending now searches a different tree (230,733 → 268,686 nodes), because threat scores apply there despite king safety fading out. Added computation and parallel branch balance also affect timing. This suite does not isolate cost per evaluation call.

Equal-time results show actual depth costs. At 500 ms with six threads, the initial position increased from completed depth 7 to 8 in all trials, while the middlegame and king-attack case fell from 7 to 6 in all trials. The closed-center six-thread 100 ms search fell from 6 to 5. The serial rook-ending 100 ms search fell from 8 to 7, and its six-thread 500 ms search reached depth 9 in four new trials (depth 10 in one) versus depth 10 in all baseline trials. Other allowances generally retained depth; pawn-ending boundary variation is in the raw data. Peak measured process RSS was 32.03 MB in this cold study, not a general memory bound.

The performance outcome is therefore mixed with substantial regressions, not an overall speed improvement. The new positional terms are implemented but untuned; only controlled strength matches can determine whether their guidance offsets the lost search depth.

The [warm-cache table](ponder.md) and [20 raw trials](ponder.json) recorded reply depth 8 serially in all trials for both evaluators after e2e4. With six threads the new evaluator reached depth 8 versus baseline depth 9 in all trials; its reply changed from d5 to e5. This is another measured cost, not evidence about which move is stronger. Peak RSS across the warm study was 66.14 MB. The 100 MB total budget and existing reservations were not changed.

## Final checks

The retained candidate passed `cargo fmt -- --check`, `cargo test --offline` (107 passed, two benchmark tests ignored), `cargo clippy --all-targets --offline -- -D warnings`, `cargo build --release --offline`, `python3 tests/http_smoke.py`, and `node tests/ui_smoke.cjs`. The UI harness passed drag/click moves and cancellation; the HTTP harness passed pondering, stale/illegal moves, mate, undo, resignation, both AI sides, and worker isolation. Archive/source hashes and all 560 measurement rows, script syntax, documentation links, and whitespace checks were verified. No unresolved correctness failures remain from this work.
