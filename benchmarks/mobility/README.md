# Piece mobility evaluation

This historical study describes the archived initial mobility implementation. The application now uses [score-preserving optimizations](../mobility-optimization/README.md); the results below are preserved.

This study isolates adding piece mobility to the engine after gradual king evaluation blending, baseline commit `80abdf6954393e0a4caffe85b602db40deedd317`. It changes evaluation intentionally; different scores and best moves between variants are expected. Existing piece-square formulas, material values, bishop-pair bonus, king endpoints, and phase weights are retained. No pruning, depth, search scheduling, time allocation, cache, or concurrency policy changes are included.

## Definition

For both colors, count destinations for each knight, bishop, rook, and queen. Friendly occupied squares, enemy pawn-attacked squares, and the enemy king's square are excluded. Sliding rays stop at the first occupied square even when it is excluded. A capturable enemy piece can count, but the ray never passes through it. Absolutely pinned knights get zero; pinned sliders count along the pin line only. Kings and pawns do not receive mobility bonuses. Their positions still affect the mobility of other pieces.

| Piece | Middlegame cp/destination | Endgame cp/destination |
|---|---:|---:|
| Knight | 4 | 4 |
| Bishop | 5 | 5 |
| Rook | 2 | 4 |
| Queen | 1 | 2 |

These are original, modest, untuned linear weights, not copied Stockfish tables. No centering penalties are applied; blocked pieces receive zero. Rooks and queens receive more weight in the endgame as files open. The signed mobility totals and king correction are blended together once using the existing phase (24 initially; knights/bishops 1, rooks 2, queens 4; capped at 24 for promotions). Rounding is toward zero and color symmetric.

This is pawn-safe pseudo-mobility, not full move legality or complete tactical safety. It does not exclude every enemy piece attack, simulate each move, demand check evasion, generate x-ray mobility, or add king/pawn mobility. No mobility or space-based pruning is introduced. Search still resolves tactical consequences.

The evaluator uses existing compiled leaper/ray geometry, builds color occupancy/pawn-attack/pin masks on the stack, and counts mobility during its piece evaluation pass. The temporary context is 64 bytes on a 64-bit target. There are no heap allocations, retained caches, locks, atomics, platform-specific paths, new assets, or dependencies. The default total server RAM budget remains 100,000,000 bytes.

## Reproduction

Run without competing builds, tests, or performance studies:

```sh
python3 benchmarks/mobility/compare.py
```

The script uses only Python standard library, Git, and Rust. It loads the baseline from the pinned revision and the candidate from archived `initial-engine.rs` (whose source hash matches the recorded original), checking shared board/geometry/cache/ordering sources against baseline. Both temporary executables are compiled before measurements with `rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`. LTO is enabled, as in the application release profile. Production search has no benchmark instrumentation.

Five repetitions use fresh processes, cold caches, seeded shuffled variant order, one thread and the automatic CPU headroom limit, and a 100,000,000-byte total memory budget. The same seven FENs and equal-depth targets as the prior blending experiment cover initial play, replies to e4/d4, a developed middlegame, a tactical promotion, a rook ending, and a pawn ending. Time allowances are 100 ms and 500 ms. Fixed-depth measurements include initialization and all completed iterations, with a 60-second guard. Timed results retain the last completed iteration, excluding unfinished work from scores/depths. Both variants have identical configured thread and memory limits.

[results.json](results.json) records all FENs, source hashes, compiler/platform, raw trials, actual cache allocation, peak RSS where supported, moves, scores, nodes, and elapsed time. Static root-alternative scores are recorded separately outside timing. [results.md](results.md) gives medians and ranges. The script verifies score agreement between serial and parallel at fixed depth within each variant and exact serial repetition of score/move/nodes. Cross-variant score equality is not required for an evaluation change.

Use `--candidate src/engine.rs --output benchmarks/mobility-optimization/with-without-follow-up.json` for an explicit comparison against the optimized application source; this requires a distinct output to preserve the original results. Otherwise, use `--output benchmarks/mobility/follow-up.json` for follow-ups; `--repetitions N` adjusts repetitions. Preserve the recorded primary results. The earlier blending driver now pins its measured candidate revision so subsequent evaluation changes do not silently alter that historical experiment.

## Results

Five repetitions across seven FENs, two thread settings (one and six), two time allowances, and equal-depth searches produced **420 measurements**. Every variant repeated its serial fixed-depth move/score/node count. Fixed-depth scores agreed between serial and parallel within each variant.

Mobility recognizes the intended positional benefit before other pieces move:

| First move | Before static score | With mobility | Added mobility contribution |
|---|---:|---:|---:|
| b1c3 | +60 | +74 | +14 |
| g1f3 | +60 | +74 | +14 |
| e2e4 | +24 | +52 | +28 |
| d2d4 | +24 | +50 | +26 |

These are whole-position static scores from the mover’s perspective, in centipawns; they exclude search. For e4, the added +28 is four bishop destinations × 5, four queen destinations × 1, and one new knight destination × 4. Both pawn moves gain more mobility than Nc3, narrowing its initial static advantage over e4 from 36 to 22 cp. That does not force the engine to prefer e4.

The initial search still chose **Nc3** in every fixed-depth and timed trial. At equal depth seven after e4, the reply changed **Nf6 → Nc6**; after d4 it remained Nf6. No other equal-depth best move changed. This is behavior evidence, not proof of better moves.

| Position | Fixed depth | Before score/move | Mobility score/move | Serial median ms before → after | Six-thread median ms before → after |
|---|---:|---|---|---|---|
| opening | 8 | +0 / b1c3 | +0 / b1c3 | 439.03 → 804.54 | 171.80 → 262.47 |
| after_e4 | 7 | +0 / g8f6 | +11 / b8c6 | 197.82 → 376.35 | 64.79 → 124.43 |
| after_d4 | 7 | +0 / g8f6 | +0 / g8f6 | 218.77 → 293.11 | 94.30 → 92.28 |
| middlegame | 6 | -10 / d5e6 | -7 / d5e6 | 521.89 → 716.43 | 254.25 → 348.78 |
| endgame | 10 | +27 / b4f4 | +20 / b4f4 | 526.53 → 839.40 | 200.61 → 298.69 |
| tactics | 6 | +479 / d7c8q | +481 / d7c8q | 190.25 → 270.78 | 64.13 → 88.95 |
| pawns | 10 | +28 / g3f3 | +28 / g3f3 | 40.32 → 48.78 | 15.93 → 17.91 |

Evaluation cost and changed search trees make this a noticeable performance regression in most positions: serial equal-depth medians increased roughly **21–90%**. Six-thread medians increased about **12–92%** in six cases; after d4 the median decreased 2%, with overlapping ranges and no convincing speedup. The raw ranges, nodes, cache allocation, and scores are in results.md/results.json. In the pawn ending at the root there are no mobility-scored pieces; building the mobility context still adds overhead, and promotions in searched continuations can change the tree.

Completed depths at short allowances also regressed in several cases:

- Initial position, one thread: 100 ms **7 → 6**, 500 ms **8 → 7**, in all five trials. Six threads retained depth seven/eight respectively.
- After e4, six threads at 100 ms: **7 → 6** in all trials. At 500 ms the new search reached depth eight in four trials and seven in one, versus seven throughout before; these depths use different evaluation and trees.
- After d4, six threads at 500 ms: **8 → 7** in all trials.
- Middlegame, six threads at 500 ms: baseline reached depth seven three times and six twice; mobility reached six throughout.
- Pawn ending, one thread at 500 ms: baseline depth 12–13, mobility depth 12. Six-thread depths were unchanged at the tested allowances.

Other settings are retained in the full report. The rook-ending and tactical-promotion samples kept the same completed depths and moves throughout, despite changed evaluations. No Elo gain is established; more positional information must be weighed against reduced depth at equal time.

Maximum observed search-process RSS was 27.6 MB, excluding browser/server. Actual cache allocation is recorded per trial; this short study does not fill the cache or establish a worst-case memory bound.

## Verification

Offline Rust tests, formatting, Clippy with warnings denied, and the release build pass. Tests cover the newly freed bishop/queen paths, friendly/enemy blockers, excluded pawn-defended blockers, diagonal/file pins, and per-piece agreement with a scalar coordinate-based oracle across existing deterministic playouts. Mirroring colors/ranks negates evaluation exactly. Existing mate, draw, cancellation, history, pondering, and serial/parallel best-move proof tests also pass.

The UI harness retains the known baseline failure at `tests/ui_smoke.cjs:414` (`13 !== 12` during drag cancellation). This change does not modify web or UI-test files.

## Limits

Short-time completed-depth changes mix evaluation cost, altered search trees, and scheduling. Equal-valued moves may differ and parallel nodes vary. Move changes alone do not demonstrate stronger play. No self-play tournament or Elo estimate is provided, and these seven positions are not a comprehensive strength test. The initial mobility weights require broader validation and tuning.
