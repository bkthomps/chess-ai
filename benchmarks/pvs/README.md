# Principal Variation Search comparison

This study compares recursive Principal Variation Search (PVS) with the engine immediately after the quiet-move ordering improvement, commit `6765813`. Both versions already have history/killer ordering and parallel root probes. The comparison measures the additional effect of PVS inside the recursive search, rather than combining it with the previous optimization.

## Implementation

The first ordered move uses the full alpha-beta window. Later moves first use a one-point window around alpha. If a probe fails low, the move cannot improve the current result. If it reaches beta, it already proves a cutoff. If it improves alpha without reaching beta, the engine searches the move again with the full window before treating it as an exact improvement. Nodes already using a one-point window perform their ordinary search without an extra probe.

PVS searches to the same target depth as before. Evaluation, quiescence, move ordering, root scheduling, cache layout/eviction, deadlines, live reporting, and CPU headroom are unchanged. PVS adds no retained tables, heap data structures, dependencies, or platform-specific code paths. The default total server RAM budget remains 100,000,000 bytes. Thinking and pondering use the same recursive search.

## Method

The benchmark compiles the baseline and current engine into the same temporary executable with identical `rustc --edition=2021 -C opt-level=3 -C codegen-units=1` flags. It refuses to run if board logic, evaluation tables, cache, or ordering source differs from the baseline. Production search has no benchmark branches. A temporary driver runs iterative deepening to a fixed depth; time-limited and pondering measurements call the ordinary search API. This build does not enable the Cargo release profile's LTO, so absolute timings/depths can differ from the application build.

Each run uses a fresh process and cold cache, the same 100 MB total budget, and a seeded shuffle of variant order. Fixed-depth timing includes initialization and all iterations through the target; a 60-second guard rejects incomplete runs. Time-limited searches receive three seconds and report the last completed depth. Three repetitions are recorded for each performance comparison. Measurements run without competing tests/builds. JSON files retain every repetition, FEN, score, node count, cache hit count, allocated cache size, peak RSS, source hash, compiler version, and platform.

The main comparison uses the automatic CPU limit (six search threads on the measured ARM64 macOS machine). A separate one-thread study uses the same targets to isolate tree-size changes from helper scheduling. The one-thread harness does not request the single-CPU yield policy because the measurement machine has multiple CPUs. Platform differences in clocks, scheduling, and allocation can change performance; the engine implementation is the same on macOS, Windows, and Linux.

## Results

PVS's clearest improvement is the middlegame: **1.75× faster** to depth seven, completing depth eight instead of seven in three seconds. The other four positions complete the same depths as before. This is an incremental gain over the preceding history/killer implementation, and it is not a uniform speedup or a playing-strength estimate.

### Automatic six-thread search

Medians of three repetitions, from [results.json](results.json). [results.md](results.md) includes each variant's timing range, nodes, cache allocation, and resident memory.

| Position | Fixed depth | Before ms | PVS ms | Before / PVS speed | Change in nodes | Completed depth in 3 seconds, before → PVS |
|---|---:|---:|---:|---:|---:|---:|
| Opening | 9 | 1,656 | 1,566 | 1.06× | +3.4% | 9 → 9 |
| Middlegame (Kiwipete) | 7 | 1,476 | 842 | 1.75× | −23.3% | 7 → 8 |
| Rook endgame | 12 | 2,403 | 2,695 | 0.89× (12.2% slower) | −6.0% | 12 → 12 |
| Tactical position | 7 | 218 | 224 | 0.97× (2.9% slower) | −1.1% | 8 → 8 |
| Pawn ending | 12 | 129 | 126 | 1.03× | +3.8% | 16 → 16 |

All three repetitions reached each completed depth shown. The middlegame fixed-depth ranges are well separated: 1,440–1,793 ms before versus 837–844 ms with PVS. The smaller differences have overlapping ranges and should not be read as strong timing conclusions. The rook-endgame PVS range is particularly broad (2,299–2,958 ms versus 2,389–2,412 ms before), although its median is worse. See the raw measurements rather than assuming a fixed speed factor.

### One-thread algorithm comparison

The same target depths, budget, and three repetitions, from [serial-results.json](serial-results.json) and [serial-results.md](serial-results.md). Each variant repeats exactly the same nodes, cache hits, score, and best move across its three runs.

| Position | Before nodes | PVS nodes | Fewer nodes | Before ms | PVS ms | Before / PVS speed |
|---|---:|---:|---:|---:|---:|---:|
| Opening | 10,096,282 | 8,436,304 | 16.4% | 2,944 | 2,456 | 1.20× |
| Middlegame | 4,222,106 | 3,223,093 | 23.7% | 1,992 | 1,377 | 1.45× |
| Rook endgame | 16,343,965 | 15,400,189 | 5.8% | 5,638 | 5,456 | 1.03× |
| Tactical position | 2,230,440 | 2,195,781 | 1.6% | 722 | 895 | 0.81× |
| Pawn ending | 851,960 | 760,851 | 10.7% | 208 | 187 | 1.11× |

PVS reduces the deterministic serial tree size in all five performance positions. The benefit is small in the tactical example, where much work is in unchanged quiescence; its initial timing ranges are also very wide and almost identical (687–971 ms before, 669–976 ms with PVS). Tree-size changes are more conclusive than that three-sample timing comparison.

A [seven-repetition tactical follow-up](tactics-followup.md), with [all raw runs](tactics-followup.json), repeats exactly those node counts. Its median times are 950 ms before and 676 ms with PVS, but both versions alternate between overlapping fast and slow timing bands (693–997 ms before, 672–996 ms with PVS). Comparing the two adjacent variant runs within each repetition gives a median speed ratio of only 1.03×. This variability does not support treating the initial 24% serial slowdown, or the follow-up's ratio of overall medians, as a stable effect of PVS. The reproducible result for that position is the modest 1.6% node reduction.

Narrow windows allow earlier cutoffs when the first ordered move is good. Re-searching unexpected improvements adds work, and probes change which cache bounds and history updates later searches encounter. The existing parallel root workers already used narrow probes before this change, limiting the incremental opportunity. Helpers share a cache and dynamic root queue, so parallel ordering, node totals, and timings can vary even with a deterministic single-thread tree. Fewer nodes therefore need not translate into a proportionate wall-time reduction, or into an extra completed ply within three seconds.

### Pondering and memory

The [pondering comparison](ponder-results.md) bounds an opening ponder to three seconds, plays human move `e2e4`, and gives the computer two seconds with its retained cache/history. Both versions complete ponder depth nine and computer depth eight in all three repetitions: **no additional completed depth in this sample**. These two depths describe different root positions. This covers one human move, not every possible ponder response.

No retained memory was added by PVS. Peak process RSS stayed below 68.0 MB in the automatic-thread depth/time study and 80.6 MB in the one-thread study, excluding the browser. The one-thread configuration has a larger cache ceiling because it reserves fewer thread stacks: about 78.6 MB versus 65.5 MB with six threads. These are observed search peaks, not an OS-enforced memory guarantee. Both versions use the same 100 MB budgeting logic.

### Correctness

All five fixed-depth benchmark scores agree between the baseline and PVS in both thread configurations. The separate [verification corpus](verification.json) compares depth-three results for 114 unique FENs from the rule-regression fixtures, including terminal positions, castling, promotions, and en passant: **114/114 scores agree**.

A new unit test independently resolves every root move, deliberately orders a poor move first, and checks that PVS finds a proven best move. It also checks exact, fail-low, and fail-high cache bounds, followed by a full-window search of the same cached position. Existing tests verify serial/parallel scores and the selected parallel move, mate scores, history-sensitive cache reuse, pondering, and cancellation. `cargo test --offline` passes 89 tests (two opt-in benchmarks ignored); Clippy with warnings denied, formatting, the release build, UI harness, and HTTP smoke test also pass. Source hashes in all saved studies match the measured implementation.

## Reproduce

Run without other CPU-heavy work. Only Rust, Git, and Python's standard library are required. Peak RSS is recorded where Python provides `wait4`; no browser process is included.

```sh
# Fixed depths and three-second searches, automatic CPU headroom:
python3 benchmarks/pvs/compare.py

# Same fixed depths on one search thread:
python3 benchmarks/pvs/compare.py --modes depth --threads 1 --output benchmarks/pvs/serial-results.json

# Three-second opening ponder, e2e4, two-second computer search:
python3 benchmarks/pvs/compare.py --modes ponder --case opening --output benchmarks/pvs/ponder-results.json

# Depth-three score agreement over the rule-regression fixtures:
python3 benchmarks/pvs/compare.py --fixtures --modes depth --threads 1 --repetitions 1 --output benchmarks/pvs/verification.json

# Extra repetitions of the noisy single-thread tactical sample:
python3 benchmarks/pvs/compare.py --modes depth --threads 1 --case tactics --repetitions 7 --output benchmarks/pvs/tactics-followup.json
```

`--quick` uses one repetition and targets two plies shallower. `--seconds`, `--repetitions`, `--threads`, `--case`, `--depth-offset`, and `--output` support other experiments. The script reuses the existing benchmark driver and report formatting. Baseline and PVS fixed-depth scores must agree; one-thread repetitions must also agree exactly on nodes, hits, score, and best move. Equally valued best moves may differ between versions.
