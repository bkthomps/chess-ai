# Aspiration window comparison

**Status: aspiration windows were removed from the application after this investigation. Move ordering and PVS remain enabled.** The results below describe the earlier experiment. Benchmark scripts compile the frozen [experimental engine](experimental-engine.rs), which is excluded from the application build, alongside the PVS baseline. References to the current/production aspiration setting below mean the setting used when these measurements were taken.

Removal restored `src/engine.rs` byte for byte to the PVS implementation at `98d2f0d`. The restored engine passes 89 Rust tests. The archived source retains the exact measured aspiration hash, and a fresh benchmark smoke run confirms that the archived experiment still compiles and agrees with PVS on the tactical score.

Aspiration windows produced **little measurable depth benefit in these tests**. With the chosen ±64-centipawn window, every three-second search and the pondering sample completed the same depths as the preceding engine. Fixed-depth timing changes were small and mixed, with overlapping ranges. Serial node counts were almost unchanged in four positions and about 2% lower in the rook endgame.

A [larger tactical follow-up](TACTICAL.md) did not reproduce the original three-trial 8.2% tactical slowdown. The default ±64 setting was essentially neutral; ±32 had a repeatable retry penalty. The original measurements below are retained as recorded, with the follow-up providing the stronger evidence for that position.

This study compares aspiration windows with commit `98d2f0d`, immediately after the PVS improvement. Both versions already have history/killer move ordering, recursive PVS, the same quiescence search, parallel root workers, and a 100,000,000-byte default server RAM budget. The measurements isolate aspiration windows from the preceding changes; they do not establish a general speed factor or playing-strength gain.

## Implementation

Starting at depth four, a search with a previous exact score initially uses that score ±64 centipawns (0.64 pawns) as its root window. Scores strictly inside the window complete the iteration. A score at or outside either boundary supplies only a bound and triggers another attempt at the same depth. The failed side widens with a doubled margin, retaining cached results and the opposite edge. When the margin reaches 1,024 centipawns, or a failed result is a mate bound, the search uses the full window. Searches without a previous exact score, shallow iterations, and predictions near mate start with the full window.

The parallel root search now accepts a bounded window. A proven beta cutoff stops claiming new root jobs, while every active helper finishes before the attempt returns. Only exact interior results can advance the completed depth or replace the last completed move. Failed windows enter the cache as upper/lower bounds. A fail-low must bound the entire root position, not just the first move; failed probes of other moves cannot be promoted to proven best moves. Cancellation during a retry leaves the caller's last completed result intact. Pondered exact scores can seed the next iteration's prediction.

This adds a few root-window integers and no retained tables, dependencies, or platform-specific paths. Evaluation, target depths, quiescence, move-ordering policy, cache capacity/eviction, CPU headroom, and reporting cadence are unchanged. The same implementation runs on macOS, Windows, and Linux.

## Method

The baseline and current engines compile into the same temporary executable using identical `rustc --edition=2021 -C opt-level=3 -C codegen-units=1` flags. The script refuses to run if board logic, evaluation tables, cache, or ordering sources differ from the baseline. The build omits Cargo release LTO, so absolute timings/depths can differ from the application build.

Each measurement uses a fresh process, cold cache, identical total RAM budget, and seeded randomized variant order. There are three repetitions per performance comparison, with no competing builds/tests. Fixed-depth measurements include initialization and every iterative-deepening pass through the target. The aspiration variant uses the production iteration/retry function; the baseline uses its existing full-window iteration. A 60-second guard rejects incomplete timing runs. Time-limited measurements call the ordinary search API for three seconds and report the last completed depth.

The main study uses the automatic thread limit (six search threads on the measured ARM64 macOS machine). The serial study uses the same targets and one thread, isolating changes in tree size from helper scheduling. A multi-CPU machine with a one-thread override does not activate the application's single-CPU yielding policy. JSON retains every run, FEN, score, node count, cache hits, allocated cache size, peak RSS, production-source hashes, compiler, and platform.

Temporary benchmark source copies also count narrow root attempts and fail-low/fail-high retries. These counters run once per root attempt, outside the node loop; the production executable has no counters or benchmark switches. Source hashes describe production files before the temporary instrumentation. In pondering runs, counters cover both the bounded warm search and the subsequent computer search. RSS excludes the browser and is recorded where Python exposes `wait4`.

## Results

### Automatic six-thread search

Medians of three repetitions from [results.json](results.json). [results.md](results.md) includes timing ranges, cache allocation, RSS, and retry counts.

| Position | Target depth | Before ms | Aspiration ms | Median time change | Completed depth in 3 seconds, before → aspiration |
|---|---:|---:|---:|---:|---:|
| Opening | 9 | 1,500 | 1,427 | 4.8% faster | 9 → 9 |
| Middlegame (Kiwipete) | 7 | 825 | 820 | 0.6% faster | 8 → 8 |
| Rook endgame | 12 | 2,353 | 2,303 | 2.1% faster | 12 → 12 |
| Tactical position | 7 | 195 | 211 | 8.2% slower | 8 → 8 |
| Pawn ending | 12 | 117 | 118 | 1.0% slower | 16 → 16 |

All three repetitions reached each displayed depth. Timing ranges overlap in every case: for example, opening 1,424–1,531 ms before versus 1,371–1,450 ms with aspiration, and tactics 194–195 ms before versus 192–222 ms with aspiration. The small median differences should not be treated as reliable universal improvements or regressions. No failed windows occurred in the five final fixed-depth or three-second searches; the performance cost of retries is visible in the smaller-window experiment below.

### One-thread search

The same targets and three repetitions, from [serial-results.json](serial-results.json) and [serial-results.md](serial-results.md). Each variant repeats exactly the same nodes, hits, score, and move across repetitions.

| Position | Before nodes | Aspiration nodes | Node reduction | Before ms | Aspiration ms |
|---|---:|---:|---:|---:|---:|
| Opening | 8,436,304 | 8,435,812 | 0.006% | 2,416 | 2,437 |
| Middlegame | 3,223,093 | 3,223,082 | <0.001% | 1,351 | 1,355 |
| Rook endgame | 15,400,189 | 15,090,449 | 2.01% | 4,946 | 4,871 |
| Tactical position | 2,195,781 | 2,195,726 | 0.003% | 658 | 661 |
| Pawn ending | 760,851 | 760,849 | <0.001% | 185 | 186 |

This shows how limited the additional pruning is with the conservative window in these positions. Parallel scheduling and shared-cache interactions explain why six-thread node/time differences need not match the serial changes. The previous PVS implementation already uses one-point probes for later moves, so aspiration mainly constrains the first search path and its descendants rather than replacing broad searches for every move.

### Choosing the initial window

The first implementation used ±32 centipawns. That helped the serial opening, but repeated score swings in the tactical position caused four failed windows by depth seven. The retained [32-centipawn study](margin32-results.md) and [serial ablation](margin32-serial-results.md) show the tradeoff:

| Position, serial target | Full-window nodes | ±32 nodes | ±64 nodes |
|---|---:|---:|---:|
| Opening, depth 9 | 8,436,304 | 7,120,469 | 8,435,812 |
| Rook endgame, depth 12 | 15,400,189 | 15,168,185 | 15,090,449 |
| Tactics, depth 7 | 2,195,781 | 2,511,051 | 2,195,726 |

The smaller window reduced opening nodes by 15.6%, but increased tactical nodes by 14.4%; its serial tactical time rose from 658 to 829 ms, and its six-thread tactical median rose from 201 to 264 ms. The narrower option is not uniformly worse, but that retry cost motivated a more conservative default. Neither setting produced extra completed depth in the three-second sample.

Before selecting ±64, a one-repetition, shallower, single-thread screen compared [±64](window64-screen.md) with [±128](window128-screen.md), using cold caches and the same positions. Neither had failed windows. ±64 visited the same number of nodes or fewer in every position (tactics differs by one node). This is a limited parameter screen, not proof of an optimal width. The final three-repetition studies above then tested ±64 at the full targets. The tradeoff sacrifices the ±32 opening gain to avoid its observed tactical retry penalty.

All 32-centipawn measurements, including [pondering](margin32-ponder-results.md) and [verification](margin32-verification.json), remain available. Experimental widths are temporary source overrides; their recorded hashes reflect the override, while the final study's hashes match production.

### Pondering and memory

[Pondering results](ponder-results.md) use three seconds of opening exploration, human move `e2e4`, then a two-second computer search retaining the cache/history. Both versions reach ponder depth nine and computer depth eight in every repetition: **no extra completed depth**. These depths refer to different root positions, and the sample covers one human move.

Aspiration adds no retained tables or cache allocation. Peak observed RSS stayed below 68.0 MB in the automatic-thread depth/time study and 78.5 MB in the serial study, excluding the browser. One thread has a larger cache allowance because fewer stacks are reserved: about 78.6 MB versus 65.5 MB with six threads. Both versions retain the same 100 MB total budgeting policy; these are observed peaks rather than an OS-enforced guarantee.

### Correctness

All five fixed-depth scores agree between the baseline and final aspiration implementation in both thread configurations. The [verification corpus](verification.json) compares 114 unique rule-regression FENs at depth four: **114/114 scores agree**, including terminal positions, promotions, castling, and en passant. Narrow windows were active in 92 positions, with 13 positions requiring retries (nine fail-low and nine fail-high events overall). Mate predictions can intentionally bypass the narrow window.

New tests check deliberately inaccurate predictions on both sides, recovery of exact scores and proven best moves, mate fallback, serial and parallel retries, restored repetition context after root cutoffs, first-move and later-move beta cutoffs, fail-low bounds covering the entire root position, and wider searches following cached bounds. Existing tests check interrupted iterations, cancellation, warm pondering, serial/parallel score agreement, and verification of the selected parallel move. `cargo test --offline` passes 91 tests (two opt-in benchmarks ignored); Clippy with warnings denied, formatting, the release build, UI harness, and HTTP smoke test pass. Final source hashes match the measured implementation, and experimental hashes match their stated width overrides.

## Reproduce

Only Python's standard library, Rust, and Git are required. Run without other CPU-heavy work.

```sh
# Fixed-depth and three-second searches, automatic CPU headroom:
python3 benchmarks/aspiration/compare.py

# Same fixed depths on one thread:
python3 benchmarks/aspiration/compare.py --modes depth --threads 1 --output benchmarks/aspiration/serial-results.json

# Three-second opening ponder, e2e4, two-second computer search:
python3 benchmarks/aspiration/compare.py --modes ponder --case opening --output benchmarks/aspiration/ponder-results.json

# Depth-four score agreement over the rule-regression fixtures:
python3 benchmarks/aspiration/compare.py --fixtures --modes depth --threads 1 --repetitions 1 --output benchmarks/aspiration/verification.json

# Reproduce the original smaller-window serial study:
python3 benchmarks/aspiration/compare.py --margin 32 --modes depth --threads 1 --output benchmarks/aspiration/margin32-serial-results.json

# Shallow candidate-window screens (one repetition each):
python3 benchmarks/aspiration/compare.py --quick --modes depth --threads 1 --margin 64 --output benchmarks/aspiration/window64-screen.json
python3 benchmarks/aspiration/compare.py --quick --modes depth --threads 1 --margin 128 --output benchmarks/aspiration/window128-screen.json
```

`--quick` performs one repetition with targets two plies shallower. `--seconds`, `--repetitions`, `--threads`, `--case`, `--depth-offset`, and `--output` support other comparisons. The verification corpus uses depth four so aspiration windows are active, with a 180-second guard for its unusually expensive quiescence positions. Fixed-depth scores must agree between versions, and serial repetitions must agree on nodes, hits, score, and best move. Equally valued best moves may differ between versions.
