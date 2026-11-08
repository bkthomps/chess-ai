# Mobility evaluation optimization

This experiment preserves the initial mobility evaluator's exact scores while reducing its execution cost. The baseline is the uncommitted mobility implementation present when this task began, archived verbatim as [initial-engine.rs](../mobility/initial-engine.rs). Its SHA-256 matches the initial mobility study's recorded candidate hash. Its preceding committed revision is `80abdf6954393e0a4caffe85b602db40deedd317`. The archive is benchmark-only and is not compiled into the application.

## Isolated changes

Each proposal was compared with the immediately preceding engine, with no concurrent builds, tests, or performance studies. Five fresh-process serial fixed-depth repetitions screened each change; small/noisy differences for stages 4 and 5 received eleven-repetition follow-ups. [patches](patches) reconstruct every stage from the archive, including intermediate implementations that have been superseded in the application.

1. **01-lazy:** Construct mobility context only on encountering the first mobility-scored piece. Pawn-only positions skip it. Later stages preserve the same fast path through the existing material phase test.
2. **02-bitmasks:** Compile knight destinations and directional ray masks, generate all pawn attacks with shifts and file masks, cache eligible destination masks by color, find pin blockers with bit scans, and count destinations with population counts. Pin detection ignores friendly pawn/king blockers because neither receives mobility scores. Sliding attacks initially use the nearest blocker to trim a ray.
3. **03-fused:** Gather occupancy, pawns, and king locations in the existing material/positional evaluation pass. Then iterate only set bits for mobility-scored pieces, removing the duplicate 64-square scan and the lazy-context branch per piece. No mobility context is constructed when phase is zero.
4. **04-lines:** Replace the slider's directional ray loops with whole-line arithmetic. On each rank/file/diagonal, subtraction reaches the first blocker; bit reversal applies the operation in the other direction. Wrapping arithmetic explicitly handles board edges. Bishops/rooks evaluate two lines, queens four. Pinned pieces are still restricted to the king's pin line. An exhaustive test checks every occupancy subset of every line on every square against independent ray walks.
5. **05-pinfilter:** Collect diagonal and orthogonal enemy-slider masks during the material pass. Skip king rays without a matching possible pinner, and skip colors without mobility-scored pieces. This also removes board-array reads from pin detection. The measured benefit is modest and position dependent; see results below.

6. **06-kingguard (screened, not retained):** An aggregate alignment check attempted to skip all pin rays at once. The five-trial screen had small, mixed changes (about -1.8% to +1.9%) rather than a convincing broad gain. It is archived for follow-up; the screen does not establish a general regression or speedup.

All five retained stages are enabled, though intermediate forms of the lazy initialization and directional slider traversal are superseded by later stages. The final application corresponds to `05-pinfilter`. Scores, weights, integer rounding, pawn-attack exclusions, blockers, absolute pins, mate distances, pruning, move ordering, cache policies, target depths, time allocation, and helper limits have the same semantics as the initial mobility implementation.

The implementation uses Rust standard-library integer operations on every platform, with no platform-specific paths, hardware-specific dependencies, heap allocations, or new locks/atomics. The 100,000,000-byte total server RAM budget and CPU headroom policies are preserved. Immutable mobility geometry occupies 6,656 bytes within the existing server-state reservation. The mobility context is 56 bytes on a 64-bit target, plus small stack arrays gathered by the evaluation pass. No retained per-thread storage is added.

## Reproduction and measurements

Run without competing CPU-intensive work. Only Rust, Git, and Python's standard library are required. Builds use `rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`; LTO is enabled, as in the application release profile. Both executables finish compiling before measurements begin. Experimental drivers and static-score diagnostics are appended only to temporary source copies; primary timing has no added node instrumentation.

```sh
# Final comparison: five trials, serial and automatic threads, fixed depth and 100/500 ms
python3 benchmarks/mobility-optimization/compare.py

# One isolated stage; stages are cumulative and patches are replayed automatically
python3 benchmarks/mobility-optimization/compare.py --variants 02-bitmasks 03-fused --threads 1 --modes depth --output benchmarks/mobility-optimization/fused-follow-up.json

# Repeat a small/noisy difference with more trials
python3 benchmarks/mobility-optimization/compare.py --variants 03-fused 04-lines --threads 1 --modes depth --repetitions 11 --output benchmarks/mobility-optimization/lines-follow-up.json

# Regression FEN corpus: static scores, serial trees, and depth-three search scores
python3 benchmarks/mobility-optimization/compare.py --fixtures --threads 1 --modes depth --repetitions 1 --output benchmarks/mobility-optimization/verification-follow-up.json

# Warm-cache search: 500 ms pondering simulation, e2e4, then a 2 s computer reply
python3 benchmarks/mobility-optimization/compare.py --case opening --modes ponder --output benchmarks/mobility-optimization/ponder-follow-up.json
```

Every timing trial starts a fresh process and cold cache, with seeded randomized variant order, equal 100 MB total budgets, and equal thread limits (one thread or the application's automatic limit). Fixed-depth timing includes initialization and all iterations through the specified depth, with a 60-second failure guard. Timed depths/scores are the last completed iteration. Warm-cache trials begin cold, then retain the cache between the simulated ponder and actual reply. The `budget_ms` field on ponder rows is the warm-up allowance, not the reply allowance.

Raw JSON records compiler/platform, flags, source hashes, FENs, repetitions, nodes, hits, completed depth/score/move, actual cache allocation, elapsed time, thread configuration, and peak RSS where available. Matching Markdown reports give medians and ranges. Static child evaluations are recorded separately, outside timing. The driver verifies cross-variant static and equal-depth score agreement, serial/parallel score agreement, and identical serial fixed-depth moves, nodes, and cache hits. Helper scheduling can vary parallel node counts and equally valued moves. Use a distinct `--output` path for follow-ups to preserve historical results.

The archived original mobility comparison also now reads its frozen source rather than silently benchmarking subsequent application changes. The earlier king-blending study remains pinned to its own measured revisions.

## Results

The final comparison contains **420 fresh-process measurements**, five repetitions for seven positions with one/six threads, fixed depths, and 100/500 ms budgets. Every fixed-depth score agrees between initial and optimized versions and between serial/parallel search. Serial fixed-depth moves, nodes, and cache hits are identical across versions and repetitions: the measured speedup comes from doing the same serial search work more cheaply.

| Position | Depth | Serial ms initial → optimized | Serial time saved | Six-thread ms initial → optimized | Six-thread time saved |
|---|---:|---|---:|---|---:|
| opening | 8 | 745.84 → 554.83 | 25.6% | 251.14 → 196.68 | 21.7% |
| after_e4 | 7 | 356.63 → 262.48 | 26.4% | 121.91 → 86.78 | 28.8% |
| after_d4 | 7 | 270.49 → 197.91 | 26.8% | 84.03 → 60.77 | 27.7% |
| middlegame | 6 | 717.34 → 534.34 | 25.5% | 346.45 → 260.99 | 24.7% |
| endgame | 10 | 811.46 → 674.91 | 16.8% | 292.06 → 244.26 | 16.4% |
| tactics | 6 | 266.17 → 213.01 | 20.0% | 84.62 → 70.25 | 17.0% |
| pawns | 10 | 47.08 → 39.23 | 16.7% | 18.57 → 15.67 | 15.6% |

Serial median time savings range from **16.7% to 26.8%**. Timings vary with position and helper scheduling; full ranges, node counts, cache allocations, and RSS are in [results.md](results.md) and [results.json](results.json). This comparison is against the initial mobility engine, not the earlier engine without mobility. It does not claim to eliminate every cost of evaluating mobility.

Equal thinking-time budgets sometimes yield another completed iteration:

- Six threads after e4 at 100 ms: initial depth six in all five trials; optimized depth seven in four trials and six in one.
- Six threads in the middlegame at 500 ms: **depth six → seven in all five trials**.
- Serial pawn ending: **depth ten → eleven at 100 ms**, and **twelve → thirteen at 500 ms**, in all trials.
- Six threads after d4 at 500 ms: optimized reached depth eight once and seven four times; initial reached seven throughout.
- Initial-position depths were unchanged: serial six/seven and six-thread seven/eight at 100/500 ms respectively. Faster evaluation did not cross an iteration boundary there.

The other primary settings did not show a lower completed-depth range for the optimized version. Time-boundary outcomes remain sensitive to scheduling; completed depth alone is not an Elo measurement.

### Optimized mobility versus no mobility

A separate direct comparison uses the engine before mobility (`80abdf6954393e0a4caffe85b602db40deedd317`, already including gradual king blending) and the final optimized mobility source. It uses the same builds, seven positions, five fresh-process repetitions, one/six threads, cold caches, and fixed-depth/100/500 ms settings: **420 further measurements**. Evaluation intentionally differs here, so tree sizes and best moves may differ. Each evaluator still agrees between serial and parallel at equal depth.

Positive percentages below mean more time with mobility; negative percentages mean less time. These are medians for reaching the same completed depth, including initialization.

| Position | Depth | Serial ms without → with | Time change | Six-thread ms without → with | Time change |
|---|---:|---|---:|---|---:|
| opening | 8 | 454.37 → 584.50 | +28.6% | 167.34 → 193.30 | +15.5% |
| after_e4 | 7 | 188.85 → 262.05 | +38.8% | 60.99 → 88.62 | +45.3% |
| after_d4 | 7 | 212.40 → 198.63 | -6.5% | 79.71 → 62.10 | -22.1% |
| middlegame | 6 | 511.56 → 534.37 | +4.5% | 254.72 → 262.33 | +3.0% |
| endgame | 10 | 509.24 → 665.79 | +30.7% | 199.84 → 229.79 | +15.0% |
| tactics | 6 | 186.90 → 212.76 | +13.8% | 53.82 → 60.51 | +12.4% |
| pawns | 10 | 38.70 → 39.18 | +1.2% | 15.46 → 15.33 | -0.8% |

Mobility still has a cost in most samples: the initial position took 29% more serial time or 16% more six-thread time; after e4 the increase was 39%/45%. After d4 it was faster because its changed evaluations reduced the searched tree enough to outweigh evaluation cost. The pawn ending was essentially neutral (small, overlapping ranges); the six-thread middlegame difference also has overlapping ranges. These are total search costs, combining evaluation overhead with different trees, not an isolated per-evaluation microbenchmark.

In the initial position at 500 ms, the serial baseline completed depth eight versus seven with mobility in all five trials. With six threads, both completed depth eight. At 100 ms both versions completed depth six serially and seven with six threads. Other timed outcomes and all ranges are in [with-without.md](with-without.md) and [with-without.json](with-without.json). Fixed time settings keep the same allowance; completed depth and move choice can differ.

Reproduce without overwriting either historical study:

```sh
python3 benchmarks/mobility/compare.py --candidate src/engine.rs --output benchmarks/mobility-optimization/with-without-follow-up.json
```

The original mobility driver defaults to the archived initial implementation; `--candidate` explicitly selects another engine and requires a distinct output path. This comparison establishes performance costs in these configurations, not a playing-strength verdict.

### Isolated stage evidence

| Comparison | Repetitions | Observation | Raw results |
|---|---:|---|---|
| Initial → lazy setup | 5 | Pawn ending 47.18 → 40.19 ms; most other differences small/mixed. | [01-lazy.json](01-lazy.json) |
| Lazy → bit masks | 5 | Broad reduction in ray/pawn/knight processing cost. | [02-bitmasks.json](02-bitmasks.json) |
| Bit masks → fused board pass | 5 | Further improvement by removing duplicate scanning. | [03-fused.json](03-fused.json) |
| Fused → whole-line attacks | 5, then 11 | Follow-up: roughly 1–7% reduction in six non-pawn cases, neutral pawn ending; effects vary by position. | [screen](04-lines.json), [follow-up](04-lines-follow-up.json) |
| Whole-line attacks → pin filtering | 5, then 11 | Follow-up rook ending 685.32 → 670.95 ms, with non-overlapping ranges; other changes mostly small/overlapping. Pawn-ending median increased 0.9%, with overlapping ranges. Retained as a modest, position-dependent improvement. | [screen](05-pinfilter.json), [follow-up](05-pinfilter-follow-up.json) |
| Extra whole-king guard | 5 | Small, mixed differences; not retained. No performance conclusion beyond this screen. | [06-kingguard.json](06-kingguard.json) |

The stage reports include full timing ranges. Small overlapping changes should not be interpreted as universal effects. Higher-repetition follow-ups were used for the two retained proposals with small initial effects.

### Warm-cache pondering

Twenty trials (five per variant/thread configuration) used a 500 ms bounded ponder of the initial position, then e2e4 and a two-second computer reply with the retained cache. Both versions completed ponder depth seven serially and eight with six threads; both then completed reply depth eight and chose Nc6 with raw score -31. **No extra completed reply depth was demonstrated in this sample.** [ponder.json](ponder.json) and [ponder.md](ponder.md) retain the full results.

The optimized search can visit more positions during unfinished iterations and grow its cache further without completing another ply. Peak search-process RSS in the warm trials was 65.6 MB (initial version 53.2 MB), excluding browser/server. This reflects increased cache utilization within the same configured limits; the small immutable geometry and stack context do not account for that difference. No memory budgeting policy was changed.

### Score verification

The separate [regression verification](verification.json) covers **114 unique FENs**, each searched serially to depth three with both versions. All scores, moves, node counts, and cache hits agree. Static evaluations of every legal root child also agree. The fixtures include terminal states, castling, promotion, and en passant. Existing independent scalar evaluation/mobility checks and search proof tests complement these comparisons.

Maximum observed process RSS in the primary study was 27.7 MB, excluding browser/server. These short studies do not fill the cache or establish a worst-case RAM bound; the existing budgeting policy and cache limits are unchanged.

## Application verification

`cargo fmt -- --check`, `cargo test --offline`, `cargo clippy --all-targets --offline -- -D warnings`, and `cargo build --release --offline` pass. The new exhaustive line-occupancy test and existing scalar mobility/evaluation oracle, color symmetry, legal/proven best moves, mate normalization, draw/terminal states, cancellation, history restoration, pondering, clocks, and undo tests pass. The HTTP smoke test passes with local socket access.

`node tests/ui_smoke.cjs` still fails at the existing drag-cancellation assertion (`tests/ui_smoke.cjs:414`, `13 !== 12`). The same failure was established on the unchanged baseline earlier in the session; this work changes no web or UI-test files.

## Limitations

Results apply to the recorded machine, positions, compiler flags, cache budgets, and thread configurations. Small overlapping timing changes are inconclusive, and fixed-time iteration boundaries are sensitive to scheduling. Faster evaluation can allow an additional completed depth; it does not establish higher Elo. This work deliberately does not change evaluation or search rules to manufacture a speedup. Pondering coverage is one human move and does not represent all possible replies or long human thinking times.
