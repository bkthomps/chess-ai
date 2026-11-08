# Parallel load-balancing comparison

This study compares leading-branch splitting with `ac1d2b3`, the current PVS engine after aspiration windows were removed. Both versions use the same move ordering, evaluation, quiescence, target depths, cache/eviction policy, 100,000,000-byte RAM budget, and automatic CPU headroom. Aspiration remains absent.

## Implementation

The preceding engine already has a dynamic queue for root moves, but helpers wait while the coordinator searches the first root move. The new implementation lets the same helpers share replies within that leading branch at deeper iterations. A parent at depth six or more can split its first child, with splitting limited to two plies below the actual root. Helpers still search their claimed jobs serially; only the coordinator creates phases.

Each nested phase finishes and joins all helpers before the parent starts its group. It never creates a pool per branch or multiplies the thread limit. A phase starts at most one helper per remaining move. Small/shallow searches use the existing path. One logical CPU still uses the original single-thread search and yielding policy; a one-thread override also bypasses parallel splitting.

The generalized queue carries the current ply, and cache store/probe operations convert mate scores at that ply. Split nodes check exact cache entries, terminal positions, and draws, while push/pop operations preserve the reversible-history context. A root result is exact only when every move is resolved; a failed probe never replaces a proven best move. Cancellation joins workers and leaves the last completed iteration intact.

There are no new dependencies or platform-specific paths. The change adds a ply integer to transient work state, and only a few bounded coordinator frames/move lists; it adds no retained per-thread tables or cache capacity. Existing thread buffers and the same memory allowance are reused. This is a limited split of the principal branch, not general work stealing at every node. A heavy later job can still leave helpers idle, and additional phase setup has a cost.

## Method

Both engines compile into the same temporary executable with identical `rustc --edition=2021 -C opt-level=3 -C codegen-units=1` flags. The default study omits Cargo release LTO for continuity with earlier reports; a separate `--lto` study uses fat LTO to check the release optimization setting. The script verifies that board logic, evaluation tables, cache, and ordering sources match the baseline. Production contains no benchmark switches.

Each trial uses a fresh process and cold cache with the same total RAM budget. Variant order is shuffled with a fixed seed. The main study uses five repetitions and the automatic limit (six search threads on the measured ARM64 macOS machine). Fixed-depth timings include initialization and iterative deepening from depth one through the target; a 60-second guard rejects incomplete trials. Three-second searches and bounded pondering call the ordinary search API. No agent-run builds/tests compete with timing studies. JSON records every run, FEN, score, best move, nodes, hits, actual allocated cache bytes, peak RSS where available, compiler/platform, and source hashes.

The one-thread control uses three repetitions at the same depths. All node counts, hits, scores, and moves must agree between variants as well as across repetitions: the serial algorithm is unchanged. A separate 15-repetition tactical study checks a smaller timing change with more samples.

Subtree-entry verification uses 114 unique rule-regression FENs at depths one through three. It compares the new parallel subtree entry at ply one with the preceding engine's serial negamax at the same ply. This deliberately exercises nonzero-ply cache normalization, draws, and terminal states in parallel, rather than running a shallow root test that would bypass the new method. Deep unit tests separately exercise actual multi-phase recursion at depths six and seven.

## Results

The change substantially reduces time to the same depth in the opening, middlegame, and pawn ending, with a smaller rook-endgame gain. The tactical result is modest and build-dependent. **Completed-depth gains at three and five seconds are occasional rather than consistent.** These are latency/throughput measurements, not an Elo or playing-strength estimate.

### Time to a fixed depth

Five-repetition medians from [results.json](results.json), with complete ranges in [results.md](results.md). The LTO column uses a separate three-repetition study, [lto-results.md](lto-results.md), rather than mixing absolute times between builds.

| Position | Target | Before ms | Balanced ms | Speedup | Time saved | LTO speedup |
|---|---:|---:|---:|---:|---:|---:|
| Opening | 9 | 1,489 | 802 | 1.86× | 46.1% | 1.34× |
| Middlegame (Kiwipete) | 7 | 814 | 486 | 1.67× | 40.3% | 1.57× |
| Rook endgame | 12 | 2,140 | 1,840 | 1.16× | 14.0% | 1.12× |
| Tactical position | 7 | 195 | 179 | 1.09× | 8.2% | 1.01× |
| Pawn ending | 12 | 112 | 73 | 1.53× | 34.7% | 1.65× |

Opening ranges are 1,386–1,562 ms before versus 760–864 ms after; middlegame ranges are 808–821 versus 484–496 ms; pawn ranges are 108–112 versus 68–75 ms. These are clearly separated. The rook-endgame ranges are also separated in this study (2,094–2,216 versus 1,760–1,993 ms). Gains remain in those four cases with LTO, though their size changes. LTO uses the release optimization setting but still runs the standalone benchmark driver, not the full browser/server application.

The tactical ranges overlap. A focused [15-repetition study](tactics-results.md) gives 196.74 → 186.95 ms, about **1.05×** (5.0% less time), with overlapping ranges of 187–238 and 177–227 ms. The LTO study is almost neutral (266.06 → 263.84 ms). The main five-run 1.09× result therefore should not be treated as a dependable tactical speed factor.

Node counts do not account for most of the large speedups: in the main study, opening nodes fall only 5%, middlegame nodes rise about 0.5%, and pawn-ending nodes fall about 0.9%. Sharing the work improves wall time without new pruning. Scheduling, move-order learning, and shared-cache contention can change node counts and total CPU work.

### Completed depths under time limits

All five three-second baseline trials complete depths 9 / 8 / 12 / 8 / 16 for opening / middlegame / rook endgame / tactics / pawns. Balanced search has the same medians; one of its five pawn trials completes depth 17. The [five-second study](time5-results.md), three repetitions per variant, gives:

| Position | Before: completed depths | Balanced: completed depths |
|---|---|---|
| Opening | 9, 9, 9 | 9, 9, 9 |
| Middlegame | 8, 8, 8 | 9, 8, 8 |
| Rook endgame | 12, 12, 12 | 12, 12, 13 |
| Tactical position | 9, 9, 9 | 9, 9, 9 |
| Pawn ending | 16, 17, 17 | 17, 17, 17 |

These samples show occasional extra plies, not a reliable additional ply on every move. Whole-depth reporting is coarse: even a large fixed-depth speedup can leave the next, much larger iteration unfinished at the deadline. The worker continues to return only its last fully resolved iteration; more unfinished nodes do not by themselves establish stronger play.

The [pondering study](ponder-results.md) uses a three-second opening ponder, human `e2e4`, and two seconds for the computer with the retained cache/history. Both versions complete ponder depth nine and reply depth eight in all five trials. The balanced search does more work within the time, but this sample shows **no completed-depth increase after pondering**.

### Where the time goes

[Root-phase diagnostics](profile-results.md) and [raw phase records](profile-results.json) instrument only temporary source copies, three repetitions. Per-phase clocks, helper lifetime counters, and observed concurrency are diagnostic evidence; the uninstrumented studies above supply the speed claims. No per-node instrumentation is added.

The first-move serial fraction is the sum of first-move segments searched without helpers, divided by total root-phase time. Nested phases contribute only their deepest unsplit first-move segment, avoiding double-counting parent time. It excludes some setup/cache-lookup time and does not measure every other source of worker idleness.

| Position | Serial-first fraction, before → balanced | Average CPU-core use, before → balanced | Maximum simultaneous helpers |
|---|---:|---:|---:|
| Opening | 65.2% → 6.4% | 2.30 → 3.97 | 5 → 5 |
| Middlegame | 67.9% → 17.9% | 1.90 → 3.56 | 5 → 5 |
| Rook endgame | 45.0% → 24.1% | 3.16 → 3.60 | 5 → 5 |
| Tactical position | 22.8% → 12.3% | 4.44 → 4.83 | 5 → 5 |
| Pawn ending | 66.4% → 18.7% | 2.00 → 3.54 | 5 → 5 |

Average CPU-core use is aggregate process CPU time divided by elapsed search time where Python provides `wait4`; it is not CPU affinity or exclusive reservation. The opening and middlegame previously spent most root-phase time on the serial first move. Sharing that branch removes much of the wait. The tactical baseline already used more of the group and had less serial-first time, leaving less opportunity.

More allowed workers are now busy, and total CPU time can rise even while elapsed time falls. The concurrency limit remains six on this machine: every diagnostic run observes at most five helpers plus the coordinator. No phase multiplies that limit. The portable CPU headroom policy and one-logical-CPU yield remain unchanged. Helper threads are still created per phase; their buffers persist between phases/searches.

### Serial control, memory, and correctness

[Single-thread controls](serial-results.md) have exactly equal nodes, cache hits, scores, and best moves across variants and repetitions in every position. Timing medians differ by roughly 0–1.8%, with overlapping ranges. The optimization is for parallel search and offers no expected benefit on a one-thread machine.

The cache ceiling remains approximately 65.5 MB for six configured threads. Peak observed RSS stays below **68.2 MB** across the six-thread studies and **78.5 MB** in the serial control, excluding the browser. The serial cache ceiling is higher because fewer stacks are reserved. Faster search can fill the existing cache sooner: the three-second opening's median RSS rises from about 36 MB to 68 MB, while the cache ceiling and total 100 MB budgeting policy are unchanged. These are observed process peaks, not an OS-enforced guarantee.

All fixed-depth benchmark scores agree, including the LTO and diagnostic builds. The [subtree corpus](verification.json) agrees on **114/114** final scores at ply one. New deep tests verify exact root scores, independent best-move scores, mate distance, terminal/draw states, and restored repetition context after splitting. Existing interrupted-iteration and pondering/cancellation checks also pass. Validation includes 90 Rust tests (two opt-in benchmarks ignored), Clippy with warnings denied, formatting, release build, UI harness, and HTTP smoke tests. Source hashes for all saved studies match the measured implementation.

## Reproduce

Only Rust, Git, and Python's standard library are required. Run without competing CPU-heavy work.

```sh
# Main comparison: fixed depths and three-second searches, five repetitions:
python3 benchmarks/parallel/compare.py

# Serial control, same target depths:
python3 benchmarks/parallel/compare.py --modes depth --threads 1 --repetitions 3 --output benchmarks/parallel/serial-results.json

# Bounded three-second opening ponder, e2e4, two-second computer search:
python3 benchmarks/parallel/compare.py --modes ponder --case opening --output benchmarks/parallel/ponder-results.json

# Focused tactical measurements:
python3 benchmarks/parallel/compare.py --modes depth --case tactics --repetitions 15 --output benchmarks/parallel/tactics-results.json

# Parallel subtree entry against serial PVS at nonzero ply:
python3 benchmarks/parallel/compare.py --fixtures --modes depth --repetitions 1 --output benchmarks/parallel/verification.json

# Check release-style LTO at fixed depths:
python3 benchmarks/parallel/compare.py --lto --modes depth --repetitions 3 --output benchmarks/parallel/lto-results.json

# A longer standard thinking-time preset:
python3 benchmarks/parallel/compare.py --modes time --seconds 5 --repetitions 3 --output benchmarks/parallel/time5-results.json

# Temporary root-phase and helper-concurrency diagnostics:
python3 benchmarks/parallel/profile.py
```

`--quick` checks the harness at shallower targets; it can bypass leading-branch splitting in shallow cases. `--seconds`, `--depth-offset`, `--case`, `--threads`, `--repetitions`, and `--output` select other experiments. CPU headroom remains the application's policy; this does not pin or exclusively reserve cores. Timings and depth thresholds will differ on other machines, and these positions do not establish an Elo gain.
