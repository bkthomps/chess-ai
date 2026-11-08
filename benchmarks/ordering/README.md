# Quiet-move ordering comparison

History and killer ordering reduce the number of nodes needed for several of these positions. In three-second searches, the opening and rook endgame complete one extra ply, and the pawn ending completes two extra plies. The tactical and middlegame examples get no depth increase and have small fixed-depth slowdowns. These are position-specific search measurements, not a playing-strength or Elo estimate.

## Implementation

The baseline is commit `5f76829`, immediately before this change. Both versions use the same evaluation, legal move generation, quiescence limits, root parallel search, cache policy, and 100,000,000-byte total server RAM budget. This implements the first proposed improvement only; internal PVS, aspiration windows, and selective depth reductions are not added.

After the cached best move, captures, and promotions, each thread tries its two recent quiet beta-cutoff moves at that ply, then sorts other quiet moves by history. A quiet cutoff rewards its move and penalizes earlier quiet alternatives, with a depth-squared bonus capped at 2,048. Signed 16-bit history counters use a bounded gravity update. Killers reset between searches; history is halved and retained, including across pondering and thinking. Equal priorities retain their original order.

History occupies 16,384 bytes per thread and the two packed killers at 266 retained plies occupy 1,064 bytes. Their combined 17,448 bytes (about 17 KiB) come from the existing 512 KiB scratch allowance by reducing retained move-buffer capacity. Six threads need 104,688 bytes for these tables. There are no new dependencies, shared locks, or platform-specific paths; CPU headroom and the cache allocation ceiling are unchanged.

## Measurements

Measured on the same eight-logical-CPU ARM64 macOS machine, with the automatic limit of six search threads, Rust `1.99.1`, and identical `rustc -C opt-level=3 -C codegen-units=1` flags. Timing studies ran without competing builds/tests. Each variant/position starts in a fresh process with a cold cache; variant order is shuffled with a fixed seed. Tables below use the median of three repetitions. Full runs, ranges, source hashes, scores, and FENs are saved in [results.json](results.json); [results.md](results.md) lists all variants.

Fixed-depth measurements include iterative deepening from depth one through the target and cache initialization. A 60-second guard fails a run if its target does not complete. Time-limited measurements call the application's ordinary search for three seconds and report the last completed depth. A ply is one side's move.

| Position | Target | Before ms | After ms | Before / after speed | Change in nodes | Depth in 3 seconds, before → after |
|---|---:|---:|---:|---:|---:|---:|
| Opening | 9 | 8,378 | 1,596 | 5.25× | −76.7% | 8 → 9 |
| Middlegame (Kiwipete) | 7 | 1,357 | 1,400 | 0.97× (3.2% slower) | +0.4% | 7 → 7 |
| Rook endgame | 12 | 3,607 | 2,418 | 1.49× | −50.8% | 11 → 12 |
| Tactical position | 7 | 191 | 206 | 0.92× (8.1% slower) | +1.8% | 8 → 8 |
| Pawn ending | 12 | 197 | 119 | 1.65× | −43.7% | 14 → 16 |

Every repetition completed the depths shown in the final column. The gains come mainly from finding cutoffs earlier and searching fewer nodes. The extra scoring and learning have a cost per node; raw nodes/second can fall even when time to the same depth improves. Where the old ordering already finds useful cutoffs, the added bookkeeping or a less useful learned ordering can cause a regression. Timings and depth thresholds will differ on other machines. Parallel scheduling also changes node counts between runs.

### Isolating history and killers

[ablation.md](ablation.md) and [ablation.json](ablation.json) compare the old ordering, killers alone, history alone, and both together, using one thread, three repetitions, and targets one ply shallower. This removes helper scheduling as an explanation for changes in tree size. Each variant repeated exactly the same nodes, hits, score, and best move across its three runs.

| Position | Old ms | Killers ms | History ms | Both ms |
|---|---:|---:|---:|---:|
| Opening, depth 8 | 2,051 | 652 | 734 | 534 |
| Middlegame, depth 6 | 533 | 533 | 543 | 541 |
| Rook endgame, depth 11 | 3,352 | 1,859 | 1,269 | 1,649 |
| Tactics, depth 6 | 195 | 197 | 194 | 195 |
| Pawns, depth 11 | 150 | 113 | 94 | 98 |

History alone is particularly effective in the rook endgame, and the combination is strongest in the opening. The six-thread study also includes history alone: it finishes the rook endgame in 1,966 ms versus 2,418 ms for both. The combination therefore does not dominate history alone. It retains the common killer/history ordering and its better opening result, with the endgame tradeoff recorded here for later tuning.

A [small exploratory screen](soft-screen.md) also tried giving killers a smaller additive history bonus (2,048 / 1,024 instead of unconditional priority over other quiet moves). At the tested shallow depths, that did not consistently improve on the combination: opening time rose from 112 to 142 ms, while the rook endgame remained slower than history alone. This screen has only one repetition, so it supports no strong timing conclusion; its exact node counts and configuration are retained in [soft-screen.json](soft-screen.json). Production uses the original combination.

### Pondering

[ponder-results.json](ponder-results.json) records a three-second opening ponder, human move `e2e4`, and a two-second computer search using the retained cache and history. Across all three repetitions:

| Version | Completed ponder depth | Completed computer depth |
|---|---:|---:|
| Before | 8 | 7 |
| After | 9 | 8 |

These depths describe different root positions; they should not be compared as if they were the same tree. This checks one human move with a warm cache, not every possible ponder response. Real pondering is cancelled when the human moves; the harness bounds it for repeatability.

### Memory and correctness

Peak process RSS across the six-thread depth/time comparisons stayed below 68.1 MB, excluding the browser. This measures actual resident memory for these searches; it does not assert a universal OS-enforced limit. Cache allocation stayed within the same approximately 65.5 MB ceiling. The ordering tables fit within the existing scratch reservation.

All fixed-depth scores agree between variants across the five benchmark positions. A separate [verification corpus](verification.json) compares the old and new ordering at depth three for 114 unique FENs from the rule-regression fixtures, including terminal positions, castling, promotions, and en passant: **114/114 scores agree**. Equal-valued best moves may differ. Unit tests also cover quiet-cutoff learning, bounded history, color isolation, tactical priority, cached-move priority, history decay, deep plies, large legal lists, serial/parallel scores, and verification of the chosen parallel move.

Validation passed: `cargo test --offline` (88 passed, two opt-in benchmarks ignored), Clippy with warnings denied, formatting, release build, the UI harness, and the HTTP smoke test including live pondering and computer replies. Saved production-source hashes match the measured implementation.

## Reproduce

Run with no other CPU-heavy work. Only Python's standard library, Git, and Rust are needed. Peak RSS is recorded where Python exposes `wait4`; timing still works when it is unavailable. The engine logic is identical across platforms.

```sh
# Full six-thread-on-this-machine comparison, including history alone:
python3 benchmarks/ordering/compare.py --variants baseline history both

# Isolate the heuristics on one thread:
python3 benchmarks/ordering/compare.py --modes depth --variants baseline killers history both --threads 1 --depth-offset -1 --output benchmarks/ordering/ablation.json

# Warm-cache pondering followed by a computer move:
python3 benchmarks/ordering/compare.py --modes ponder --case opening --output benchmarks/ordering/ponder-results.json

# Rule-regression score agreement:
python3 benchmarks/ordering/compare.py --fixtures --modes depth --threads 1 --repetitions 1 --output benchmarks/ordering/verification.json

# Exploratory alternative killer weighting:
python3 benchmarks/ordering/compare.py --quick --modes depth --variants history both soft --threads 1 --output benchmarks/ordering/soft-screen.json
```

`--quick` performs one repetition at shallower targets; `--seconds`, `--repetitions`, `--threads`, `--case`, and `--depth-offset` support other experiments. Temporary source copies disable individual heuristics for ablation. The production executable has no benchmark branches or runtime heuristic switches. The benchmark adds only a temporary driver to access fixed-depth search; production time-limited and pondering tests call the regular engine entry point.
