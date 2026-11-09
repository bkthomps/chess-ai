# Score-preserving engine optimization

Baseline revision: `bf4342c2f2de14c24d3f504e583e179905ebf83b`. The frozen [baseline](baseline-engine.rs) is the immediately preceding application engine, including central space, pawn structure, piece–pawn relationships, threats, and king safety. Each proposal is isolated against the preceding retained implementation. Archived engines are reproduction artifacts, not application modules.

## Method

The dependency-free [comparison script](compare.py) uses the existing memory study's temporary driver. All executables compile before timing with `rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`, matching the application's release LTO and single codegen unit. No builds, tests, or other performance studies run during timing. Each measurement starts a fresh process, with cold caches and seeded randomized variant order. Raw JSON records source hashes, shared-source revision/hashes, compiler, platform, FENs, target depths, repetitions, actual threads, total 100,000,000-byte budget, actual transposition allocation, elapsed time, scores, best moves, nodes, hits, and peak RSS where supported. Tables preserve medians and ranges.

`--require-equivalent` verifies all legal root-child static evaluations, equal-depth scores across variants/threads, and identical serial moves/nodes/transposition hits. Parallel search work may differ with scheduling. Timed searches report only fully completed iterations. A ponder trial warms the ordinary search for 500 ms, plays e2e4 (or the documented fallback in the driver), then measures a two-second reply using retained caches/history. This is a reproducible bounded approximation of the application's cancellable pondering.

## Initial screens

Nine fresh trials per variant/position, serial, equal completed depths:

- [Pawn cache](01-pawn-cache.md) / [raw measurements](01-pawn-cache.json): 1,024 entries validate both pawn bitboards and retain raw middlegame/endgame pawn terms. Its 24,576 bytes per thread are charged to existing scratch. Small mixed improvements require confirmation; this isolated cache is not retained alongside the complete-evaluation cache.
- [Packed sorting](02-packed-order.md) / [raw measurements](02-packed-order.json): pack priority and move into one word while sorting, then decode. Stable equal-priority ordering is preserved, including the fallback for unusually large move lists. Mixed results show no consistent gain; rejected.
- [Complete evaluations](03-evaluation-cache.md) / [raw measurements](03-evaluation-cache.json): 1,024 entries validate the full position hash and all 64 piece squares. The score is stored relative to White, so turn changes cannot reuse the wrong sign. Static scores contain no repetition, halfmove, terminal, mate-distance, or search-bound information. Search retains all existing terminal/draw checks before consulting this cache. The 81,920-byte table fits inside each thread's existing scratch allowance, reducing retained move-buffer capacity by the same amount.

The first two screens use the previous eight-position corpus; complete-evaluation studies also include the recent king-attack position. Earlier rejected arithmetic and one-entry-cache experiments remain in their original directories.

## Capacity and scheduling screens

[Capacity measurements](04-cache-size.md) / [raw data](04-cache-size.json) compare 4,096 entries with the preceding 1,024-entry implementation, nine serial trials at each of nine positions. The larger table reduced median equal-depth time by 1.1–3.5% in every sampled position. Its 327,680 bytes per thread are deducted from the same scratch cap, leaving 179,160 bytes for retained move buffers after the 17,448-byte ordering tables. The [retained cache engine](retained-cache-engine.rs) adds independent-oracle and memory-budget tests; its release implementation is equivalent to the [larger-table screen](large-evaluation-cache-engine.rs).

[Earlier splitting](05-earlier-split.md) / [raw data](05-earlier-split.json) test starting leading-branch splitting at depth five instead of six against the retained cache engine. Seven repetitions include serial controls and automatic threading (six threads here). All fixed-depth scores agree; serial moves/nodes/hits are unchanged. Automatic-thread median differences range from -14.0% to +4.5%. Middlegame, closed-center, and king-attack cases improve 10.5–14.0%, but the opening regresses 3.0% and rook/pawn endings regress about 4.5%. This mixed proposal was rejected; no fixed-time or pondering promotion study was warranted. Existing phase joining, concurrency limits, and the depth-six policy remain unchanged.

The independent pawn cache was not promoted: its smaller benefits were less consistent than complete-evaluation caching. Combining both caches has not been established as beneficial. Packed sorting was also rejected. No evaluation weights, pruning, search targets, transposition replacement policy, thread limits, or total memory budget change.

## Retained-cache confirmation

[Raw confirmation](06-confirmation.json) and [all ranges](06-confirmation.md) contain 756 fresh-process measurements: seven trials per variant, nine positions, serial and automatic threading, equal depth and 100/500 ms allowances. All static scores, fixed-depth scores across threads/variants, and serial moves/nodes/transposition hits agree exactly.

| Position | Serial equal-depth time | Six-thread equal-depth time |
|---|---:|---:|
| opening | -5.0% | -3.9% |
| after_e4 | -4.9% | -1.6% |
| after_d4 | -5.5% | -4.2% |
| middlegame | -4.9% | -5.5% |
| closed_center | -6.8% | -5.1% |
| endgame | -9.4% | -9.5% |
| tactics | -5.6% | -5.4% |
| king_attack | -3.9% | -2.5% |
| pawns | -8.2% | -10.1% |

Negative changes mean faster. Serial medians improve 3.9–9.4%; six-thread medians improve 1.6–10.1%. Small parallel differences still overlap scheduling variation; use the recorded ranges rather than treating every percentage as universal.

Equal-time results usually complete the same depth. With six threads at 100 ms, closed-center depth six is reached in 2/7 cached trials versus 0/7 baseline trials. At 500 ms the rook ending reaches depth ten in 5/7 cached versus 4/7 baseline trials. The serial pawn ending reaches depth thirteen in 2/7 cached versus 0/7 baseline trials at 500 ms. At 100 ms, one cached pawn-ending trial completes only depth nine while all other trials of both variants complete depth ten; this outlier is preserved rather than excluded. Short-budget outcomes cannot guarantee an extra ply on every move.

Peak recorded RSS in confirmation is 36.29 MB. The transposition allocation ceiling and total reservations remain unchanged; this workload's RSS is not a universal bound.

## Pondering and short-budget follow-up

[Warm-cache results](07-ponder.md) / [raw data](07-ponder.json) include seven trials per variant/thread setting. Both variants complete warmup depth seven serially or eight with six threads, then reply depth eight in every two-second trial. No extra completed reply ply is demonstrated. Peak recorded warm-study RSS is 59.18 MB.

The [21-trial pawn-ending follow-up](08-pawn-time-follow-up.md) / [raw data](08-pawn-time-follow-up.json) keeps the original anomalous result intact and increases repetitions. At 100 ms, baseline completes depth ten in 21/21 trials; the cached engine completes depth ten in 20/21 and depth nine in one. At 500 ms, baseline completes depth twelve in 21/21 trials while the cached engine completes depth thirteen in 21/21. Thus the longer allowance demonstrates an extra completed ply on this position, while the short-budget outlier recurs. Its cause is not established; faster medians do not guarantee a higher depth on every timed search.

The static evaluator itself is byte-for-byte unchanged from baseline (SHA-256 `ddc94ffca331048e4bf88ca130f7940ed97bf0fcd3f032ea910a9bb11ef1d244` for the function text). The retained engine archive matches application source exactly, including its verification tests.

## Verification

[Regression-corpus results](09-fixture-verification.md) / [raw data](09-fixture-verification.json) verify all 125 unique FENs from the move, perft, castling, terminal, and draw fixtures at depth three. All static scores and fixed-depth scores/moves/nodes/transposition hits agree exactly with baseline, including no-best-move terminal positions. Across the nine saved studies there are 1,982 fresh-process measurements.

Application checks passed: `cargo fmt -- --check`, `cargo test --offline` (108 passed, two opt-in benchmarks ignored), `cargo clippy --all-targets --offline -- -D warnings`, `cargo build --release --offline`, `python3 tests/http_smoke.py`, and `node tests/ui_smoke.cjs`. Tests cover the independent evaluator over playouts and mirrors, forced hash/index collisions, turn changes, special moves, the combined retained scratch cap, serial/parallel best-move proof, mate normalization, terminal/draw handling, cancellation, history restoration, pondering, clocks, and undo. Source hashes, benchmark syntax, relative documentation links, and whitespace were checked.

## Reproduce

Run without competing CPU-intensive work. Use distinct output paths to preserve historical measurements.

```sh
python3 benchmarks/engine-optimization/compare.py --baseline benchmarks/engine-optimization/baseline-engine.rs --candidate benchmarks/engine-optimization/pawn-cache-engine.rs --require-equivalent --threads 1 --modes depth --repetitions 9 --output benchmarks/engine-optimization/pawn-follow-up.json
python3 benchmarks/engine-optimization/compare.py --baseline benchmarks/engine-optimization/baseline-engine.rs --candidate benchmarks/engine-optimization/packed-order-engine.rs --require-equivalent --threads 1 --modes depth --repetitions 9 --output benchmarks/engine-optimization/sorting-follow-up.json
python3 benchmarks/engine-optimization/compare.py --baseline benchmarks/engine-optimization/baseline-engine.rs --candidate benchmarks/engine-optimization/evaluation-cache-engine.rs --require-equivalent --threads 1 --modes depth --repetitions 9 --output benchmarks/engine-optimization/evaluation-follow-up.json
```

The full retained-cache comparison uses the baseline and `retained-cache-engine.rs`, `--threads 1 0 --modes depth time --repetitions 7`. Pondering uses `--case opening --modes ponder --threads 1 0 --repetitions 7`; the pawn follow-up uses `--case pawns --modes time --threads 1 --repetitions 21`.

For capacity and scheduling reproduction, pass `evaluation-cache-engine.rs` / `large-evaluation-cache-engine.rs`, or `retained-cache-engine.rs` / `earlier-split-engine.rs`, respectively. Use `--threads 1 0 --repetitions 7` for the scheduling screen. `--fixtures --modes depth --threads 1 --repetitions 1` verifies the regression corpus, including terminal positions with no best move.

Timing, throughput, and additional completed plies do not establish Elo or playing strength. This study preserves evaluation values rather than tuning them. Samples do not prove a universal speedup or an OS-enforced memory limit.
