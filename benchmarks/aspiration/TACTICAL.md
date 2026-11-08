# Investigating the tactical sample

**Status: the application now uses PVS without aspiration windows.** This report preserves the investigation that led to removal. Reproduction scripts use the archived [experimental engine](experimental-engine.rs), so the results and variant names below refer to the implementation at the time of measurement.

This follow-up tests whether the original three-repetition, six-thread median slowdown (195 ms to 211 ms, about 8.2%) is reproducible, and whether aspiration adds useful search work reduction in this position. Production engine sources remain unchanged during the investigation.

**The 8.2% slowdown was not reproduced. The default ±64 setting performs essentially the same as PVS here, with no extra completed depth.** The smaller ±32 setting has a repeatable retry penalty. These findings weaken the case for keeping aspiration as a useful optimization in this engine, rather than establishing a general 8% regression.

The position is `rnbqk2r/pp1Pbppp/2p5/8/2B5/8/PPP1NnPP/RNBQK2R w KQkq - 1 8`. Both versions choose the promotion capture `d7c8q` at the measured fixed depths. This is one position; conclusions do not apply to every tactical position.

## Method

The primary study compares PVS at `98d2f0d` with the current ±64-centipawn aspiration implementation, using the same two-variant executable generator as the original study. It uses fresh processes, cold caches, the same 100 MB total memory budget, and identical optimized `rustc` flags. Automatic headroom permits six search threads on this machine. The one-thread override also runs on this multi-CPU machine.

There are 31 measured pairs at depth seven for each thread count, nine pairs at depth eight, seven pairs of serial three-second searches, and eleven pairs of parallel three-second searches. Each stage starts with one discarded warm-up pair. Primary pair order is balanced between baseline-first and aspiration-first (differing by at most one pair), then shuffled with a fixed seed. No other agent-run builds, benchmarks, or tests compete with these measurements.

The report retains both median times and the median per-pair percentage cost. A positive paired cost means aspiration took longer. Descriptive 95% percentile bootstrap intervals use 10,000 resamples of the measured pairs. They summarize these runs, assuming pairs are suitable resampling units; serial dependence, machine conditions, compiler layout, and scheduling limit generalization. Hardware affinity and CPU frequency are not controlled. The temporary build uses the same optimization flags as the previous study and omits Cargo release LTO, so it is not an exact measurement of the full running application.

CPU time, process wall time, context-switch counts, and peak RSS are recorded where Python exposes `wait4`, without any platform-specific engine code. Engine-reported elapsed time remains the primary timing metric. Timed searches include the unfinished iteration in node counts but report the last completed depth. Fixed-depth scores must agree; single-thread repetitions must also match exactly on nodes, hits, score, and move.

## Controls and diagnostic builds

Separate nine-repetition control groups at depth seven compare:

- `baseline`: the preceding PVS engine.
- `current_full`: the current engine with only the aspiration prediction disabled in a temporary source copy, keeping the new root-window machinery.
- `aspiration`: the production ±64-centipawn setting.
- `margin32` and `margin128`: temporary smaller/larger initial windows.

These control variants share an executable and run in shuffled groups, so their timings should be compared within this study rather than mixed with the original executable's absolute times. They separate the new machinery, window width, and retry behavior without changing the project engine.

Additional single-thread diagnostic builds count negamax calls, quiescence calls, internal PVS probes, full-window PVS re-searches, depth-zero calls, and calls entering with one-point windows. They record cumulative counters and scores after each completed depth. Per-node instrumentation changes runtime, so **diagnostic timings are not used as performance evidence**. Their deterministic node totals are checked against the uninstrumented runs where available.

## Repeated timings

[Timing measurements and ranges](tactical-timing.md), [all raw trials](tactical-timing.json). Intervals below describe the median paired percentage cost, not the ratio of the two displayed medians; these statistics can differ.

| Search threads | Completed target | Measured pairs | PVS median ms | ±64 median ms | Paired cost, median | Descriptive bootstrap 95% range |
|---|---:|---:|---:|---:|---:|---:|
| 1 | 7 | 31 | 657.85 | 658.00 | −0.36% | −0.62% to +0.35% |
| 6 | 7 | 31 | 192.06 | 190.17 | −1.45% | −2.14% to +0.08% |
| 1 | 8 | 9 | 3,491.54 | 3,505.43 | +0.24% | −0.06% to +1.51% |
| 6 | 8 | 9 | 1,180.09 | 1,181.05 | +0.41% | −0.94% to +1.70% |

Every interval includes zero. These measurements do not support a consistent 8% cost, or a useful speedup. At depth seven, the original aspiration range (192–222 ms) contained the preceding engine's roughly 194–195 ms results. Its three-trial median was sensitive to run variation; the much larger balanced study does not sustain that estimate. The original runs did not capture enough system information to attribute their difference to a specific scheduling/frequency event.

The new process CPU measurements are also close: six-thread depth seven uses median total CPU time of 882 ms for PVS versus 880 ms with aspiration; depth eight uses 4,758 versus 4,749 ms. Aggregate CPU time can exceed elapsed time because helpers run concurrently. These measurements do not reveal an extra 8% computational cost from the default window.

With three seconds per move, both versions complete depth seven in **7/7 serial trials**, and depth eight in **11/11 six-thread trials**. There is no demonstrated depth gain here. A timed search's total node count includes unfinished work, so nodes/second alone is not a strength measure.

## Window width and added machinery

[Control measurements](tactical-controls.md), [raw data](tactical-controls.json). Nine measured groups per thread setting at target depth seven:

| Variant | Serial median ms | Serial nodes | Root retries | Six-thread median ms |
|---|---:|---:|---:|---:|
| Preceding PVS | 660.14 | 2,195,781 | 0 | 200.59 |
| Current machinery, full windows | 665.41 | 2,195,781 | 0 | 193.36 |
| Current ±64 | 659.71 | 2,195,726 | 0 | 192.10 |
| ±128 | 655.48 | 2,195,731 | 0 | 196.64 |
| ±32 | 828.37 | 2,511,051 | 4 | 252.61 |

Disabling only aspiration prediction in the current temporary engine yields the same deterministic serial tree as the preceding PVS implementation. It does not reveal a substantial cost from the new root-window machinery. Its serial paired cost is +0.46%, with a descriptive range spanning −0.97% to +1.56%. The ±64 and ±128 controls are similarly close to the baseline.

The ±32 result is different: four failed windows, **14.4% more nodes**, and median paired time costs of **25.3% serial** and **28.1% parallel**. Its serial descriptive range is +22.8% to +27.0%, and the parallel range +22.1% to +41.4%. This is repeatable extra work from retries, consistent with the original smaller-window experiment.

The root score explains those retries. The diagnostic trace for the same position gives:

| Depth | Exact score, centipawns | Change from previous depth |
|---|---:|---:|
| 3 | 559 | — |
| 4 | 503 | −56 |
| 5 | 539 | +36 |
| 6 | 479 | −60 |
| 7 | 539 | +60 |
| 8 | 485 | −54 |

The ±32 window misses the score in each of depths four through eight, whereas ±64 contains every change. Through depth seven that means two fail-low and two fail-high retries; ±64 has none. This is an evaluation oscillation as deeper replies are included, with the root position unchanged.

## Why the default window saves almost nothing

[Diagnostic counters](tactical-profile.md), [per-depth traces and raw counters](tactical-profile.json). These are single-thread depth-eight builds; their node totals match the corresponding uninstrumented timing runs. Nodes count search visits, including repeated visits and both negamax/quiescence calls, rather than unique board positions.

| Counter, cumulative through depth 8 | PVS | ±64 aspiration |
|---|---:|---:|
| Total counted visits | 7,477,203 | 7,477,148 |
| Positive-depth negamax calls | 1,110,402 | 1,110,402 |
| Positive-depth calls with a one-point window | 1,110,356 | 1,110,356 |
| Quiescence calls | 4,589,737 | 4,589,686 |
| PVS probes | 1,657 | 1,657 |
| PVS full-window re-searches | 22 | 18 |

Over **99.99% of positive-depth search calls already have a one-point window** under PVS. Aspiration has little left to tighten in this tree. The conservative setting saves four PVS re-searches and 55 total visits, a **0.00074%** reduction through depth eight. Quiescence itself accounts for about 61.4% of the engine's counted visits; it also inherits one-point windows for nearly all calls. Much of the remaining count consists of depth-zero negamax calls and the existing narrow search.

The final depth-eight iteration visits **5,281,422 nodes in both versions**. All 55 saved visits occurred in earlier iterations. At depth seven the savings are also only 55 visits out of about 2.2 million (0.0025%). This explains the near-identical timings more directly than the initial three-sample median did.

The narrow ±32 diagnostic reaches 12,557,968 visits through depth eight, with five failed windows, versus PVS's 7,477,203. That is about **68% more visits** in this diagnostic trace. Its cumulative depth-seven count agrees with the repeatable control runs above. No instrumented timing is used to estimate that cost.

## Decision

The current setting does not have a reproducible tactical slowdown of 8%, but it also contributes essentially no useful pruning or depth gain in this sample. Together with the original five-position study's limited benefits, the evidence does not make a strong case for its added complexity. I would remove aspiration for now and keep the move-ordering/PVS improvements. Engine sources were not changed during this analysis.

## Reproduce

```sh
# Run all studies sequentially:
python3 benchmarks/aspiration/tactical.py

# Individual studies:
python3 benchmarks/aspiration/tactical.py --study timing
python3 benchmarks/aspiration/tactical.py --study controls
python3 benchmarks/aspiration/tactical.py --study profile
```

The driver writes `tactical-timing.json`, `tactical-controls.json`, and `tactical-profile.json`, with Markdown tables. Only Python's standard library, Git, and Rust are required. Source hashes identify each production/temporary variant. `--quick` checks the harness with one measured group per stage; it is not sufficient for the timing conclusions.

Validation: 196 primary timing trials, 90 control trials, and three diagnostic traces are saved. Fixed-depth scores agree across variants; serial repetitions are deterministic. Diagnostic depth-seven totals match the serial control trials, and default/baseline depth-eight totals match the primary trials. The engine's source hash matches the preceding aspiration study exactly; Python syntax and whitespace checks pass. Timing builds include 22 additional discarded warm-up runs.
