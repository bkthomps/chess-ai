# Check-evasion destination filter

The move generator already scans king rays for checks and absolute pins. It now also collects destinations that could capture or block every checker. Pawn, knight, and king checks allow only the checking square; slider checks also include the intervening squares. Intersecting the masks handles multiple checks. Ordinary non-king replies outside that mask are rejected before making a temporary board and checking attacks.

Every surviving candidate still receives the existing legality validation. King moves and en passant bypass the destination filter because they can change attack geometry or capture a pawn on a different square. This is an exact move-generation optimization: it preserves generated move order, evaluation, search depth, pruning rules, quiescence limits, cache identity and allocation, the total 100,000,000-byte RAM budget, and CPU/thread policy. It adds one local mask and no heap allocation, retained state, synchronization, or per-node instrumentation. Thinking and pondering use the same code.

The immediately preceding engine is revision `162cd89` (full revision and all source SHA-256 hashes are recorded in each JSON). [candidate.patch](candidate.patch) is the frozen candidate, reconstructed from that baseline for reproduction; rejected sorting and evaluation ideas are archived separately under [lazy-ordering](../lazy-ordering/README.md) and [king-pawn-lanes](../king-pawn-lanes/README.md).

## Measurements

The destination filter is retained. Repeated fixed-depth measurements favor it across the opening-followup, middlegame, closed-center, rook-ending, and tactical samples. The pawn-only sample is effectively neutral in the initial screen and slightly slower in confirmation; it gets little benefit from filtering checks. Starting-position results vary substantially between runs.

| Position | Initial serial time change | Confirmation serial | Confirmation six threads |
|---|---:|---:|---:|
| starting position | +1.0% | -6.7% | -14.2% |
| after e4 | -6.2% | -7.8% | -9.6% |
| after d4 | -2.7% | -2.0% | -0.8% |
| middlegame | -8.4% | -12.4% | -10.3% |
| closed center | -13.5% | -7.2% | -13.8% |
| rook ending | -4.8% | -9.3% | -7.2% |
| tactics | -14.6% | -12.9% | -5.3% |
| pawn ending | -0.5% | +3.4% | +1.1% |

These are median paired time changes at the same completed depth; negative is faster. Individual pairs sometimes regress, and ranges overlap broadly in several cases. For example, confirmation serial tactics ranges from 10.1% to 22.9% faster, while the pawn-ending pairs range from 34.8% faster to 29.5% slower. Small changes there are inconclusive.

Equal one-second budgets usually complete the same depth. Across 144 paired trials there are 11 deeper, 125 tied, and 8 shallower candidate results. In serial tactics the candidate completes depth seven in all nine trials, while baseline completes six in three trials and seven in six. Conversely, starting-position serial median depth falls from eight to seven in this noisy sample (three deeper, two tied, four shallower). Default six-thread starting-position trials all complete depth eight for both variants. The data supports a reduction in equal-depth search cost, with occasional extra plies; it does not establish a uniform fixed-time depth increase.

Pondering uses seven pairs in each of three positions with one and six threads: 42 pairs total. Completed reply depth is higher in two candidate trials, tied in forty, and never lower in this sample. Median depths are unchanged in every group. The largest observed process RSS across the retained-candidate studies is 69.93 MB, and maximum actual cache storage is 65.46 MB; the configured total budget remains 100 MB. These measurements do not replace the existing reservation/budget tests.

See [paired comparisons](analysis.md), the [initial serial screen](screen.md), [confirmation medians/ranges](confirmation.md), and [pondering results](ponder.md). Each Markdown table has corresponding raw JSON containing every trial, FEN and requested depth, compiler/platform, source hashes, configured/actual threads, total budget, actual cache allocation, and peak RSS where available.

All timing uses optimized uninstrumented search in fresh processes, with cold caches except the explicitly warmed pondering mode. Variant order is shuffled with seed 1711. Both executables finish compiling before measurement. No task-run builds, tests, or competing benchmark studies overlap measurement. Compiler flags are `--edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`; LTO is enabled as in the application release profile.

The initial screen uses eleven serial pairs per position. Confirmation includes nine pairs at equal completed depths and nine pairs under equal one-second thinking budgets, with one thread and the machine's default six search threads. The fixed-depth driver performs iterative deepening with full-window root selection, preserving the historical benchmark approach. Fixed-time/ponder modes invoke application search, including the existing transient opening move-selection margin. Do not equate these different root-selection workloads. Completed-depth score agreement is required across variants and thread configurations. Serial fixed-depth scores, best moves, nodes, and transposition-table hits must match exactly.

Paired timing summaries use the median of candidate/baseline ratios within each repetition. Raw medians and full ranges are also retained. The machine exhibits substantial run-to-run variation; small timing differences and single extra-depth outcomes are inconclusive. Parallel node counts and ordering can vary with scheduling. Fixed-time node totals include unfinished work; depth counts only completed iterations. An improvement in time or node throughput does not guarantee an extra ply in every position, and these measurements make no Elo claim.

## Verification and reproduction

The legal/tactical generator is checked against the independent make-and-check reference over seeded playouts. Added starting cases exercise double check, en passant capturing a checking pawn, en passant blocking a bishop, and promotion captures/interpositions. Existing perft, repetition, terminal/draw, mate normalization, serial/parallel score and best-move proof, cancellation, history restoration, clocks, undo, and server/UI regression tests cover the surrounding behavior.

Final application verification passed: `cargo fmt -- --check`, `cargo test --offline` (112 tests passed, two explicit benchmarks ignored), `cargo clippy --all-targets --offline -- -D warnings`, `cargo build --release --offline`, `python3 tests/http_smoke.py`, and `node tests/ui_smoke.cjs`. The release executable was rebuilt before HTTP verification. `git diff --check`, benchmark Python syntax checks, local study links, and the match between application source hashes and the measured candidate also passed.

Rust and Python standard libraries only; use separate output paths to preserve historical data:

```sh
python3 benchmarks/check-evasions/compare.py --threads 1 --modes depth --repetitions 11 --output benchmarks/check-evasions/screen-follow-up.json
python3 benchmarks/check-evasions/compare.py --threads 1 0 --modes depth time --budget 1000 --repetitions 9 --output benchmarks/check-evasions/confirmation-follow-up.json
python3 benchmarks/check-evasions/compare.py --threads 1 0 --modes ponder --budget 500 --cases opening middlegame endgame --repetitions 7 --output benchmarks/check-evasions/ponder-follow-up.json
python3 benchmarks/check-evasions/analyze.py
```

Thread limit zero selects the portable application default, reserving roughly one-quarter of logical CPUs. Ponder trials warm the original position for 500 ms, play one deterministic legal human move, then measure a two-second reply with the same cache. Each pair uses separate fresh processes. Benchmark entry points are appended only to temporary source copies; the application has no benchmark switches. `analyze.py` summarizes the three archived JSON files; follow-up files can be inspected using the equivalent grouping logic or their generated Markdown tables.
