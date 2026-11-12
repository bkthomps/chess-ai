# Fixed cache ceilings

Historical configuration: these measurements used the fixed 60 MB application default with a 100 MB total server budget. The application default was subsequently changed at the user’s request to a fixed 100 MB cache ceiling. Its thread policy retains the original CPU headroom rule (roughly 75% of logical CPUs, at least one thread, capped at 64); the total-server planning budget and eight-thread RAM cap were removed. The measurements and `application.patch` below preserve the tested configuration. References below to the application default describe the configuration at measurement time.

This study varies the cache ceiling directly: 60, 80, 100, 120, 140, and 160 decimal MB. These are **cache** ceilings, not total-server budgets. The application default remains a 100 MB total-server budget with a fixed 60 MB cache ceiling and at most eight search threads. Larger experimental caches do not fit that application budget.

Run from the repository root:

```sh
python3 benchmarks/fixed-cache/compare.py --repetitions 5
python3 benchmarks/fixed-cache/report.py
```

Use `--resume` to continue a partially collected output after excluding any interrupted trials. It checks the source hashes and configuration before reusing measurements. New trials record UTC start and finish timestamps. `interrupted-sleep.json` preserves the excluded collection after a reported laptop sleep; the accepted dataset records the exclusion scope.

Use `--output` for follow-ups so the original measurements survive. `report.py` summarizes `results.json`. A short smoke run can use:

```sh
python3 benchmarks/fixed-cache/compare.py --repetitions 1 --depths 6 --cases opening endgame --threads 1 6 --output benchmarks/fixed-cache/smoke.json
```

The driver uses temporary source copies and standard-library Python/Rust only. It appends the existing memory-study driver, updated for the current Search fields, and overrides only the table ceiling. Production search gets no benchmark instrumentation. Both serial and six-thread searches use the same evaluation, ordering, cache policy, and complete iterative deepening through the requested depth. Fixed-depth searches have a 30-minute safety deadline and fail rather than report unfinished work as completed. The depth driver measures raw search scores; it does not apply the application's final opening-move preference selection.

Each trial starts a fresh process with a cold cache. Size order is shuffled with a recorded fixed seed (4281). Optimized binaries use `opt-level=3`, `codegen-units=1`, and `lto=fat`, matching the release profile's LTO policy. No builds, tests, or other agent benchmarks run concurrently with measurement. Five repetitions cover each size at depths 6, 8, and 10 in opening, tactical middlegame (Kiwipete), and rook endgame positions. Two-second searches measure completed depth under equal thinking-time allowances. One-thread runs provide a serial control. Pondering and long warm-cache sessions are outside this study.

`results.json` records the base revision, source SHA-256 hashes, compiler, platform, FENs, flags, configured thread count, requested cache ceiling, actual allocation including page bookkeeping, nodes, hits, score, move, time, and peak RSS where available. The temporary constructor uses an experimental 1 GB total budget before replacing its table with the requested direct ceiling; no measured run allocates a 1 GB table. Cache allocation starts at 4 MiB and grows on collision pressure; a larger ceiling is not preallocated. Thus this tests the existing on-demand cache with fixed ceilings, not fully preallocated tables. RSS includes allocator and process memory and excludes the browser.

Fixed-depth scores must agree across sizes and serial/parallel configurations. Serial nodes, hits, move, and allocations must repeat for each size; parallel work may vary with scheduling and equal-valued moves may differ. Timing tables retain full ranges as well as medians. Differences where all sizes perform the same work below 60 MB are timing noise, not evidence for a larger cache. Position-dependent wins at deeper searches do not establish a universal optimum or an Elo improvement.

`pilot.json` is a separate preliminary smoke study, not part of the main aggregate.

## Findings

All 720 main trials and 30 additional confirmation trials completed. Every fixed-depth score agrees across sizes, repetitions, and thread settings. Serial search-work counts and moves repeat exactly for each size. Full medians, ranges, and RSS are in [results.md](results.md); raw measurements are in [results.json](results.json).

| Target depth | Recommended cache ceiling | Evidence |
|---|---|---|
| 6 | 60 MB | All sampled searches allocate only 4.21 MB; larger ceilings cannot help capacity. |
| 8 | 60 MB | All sampled searches allocate at most 33.69 MB; larger ceilings cannot help capacity. |
| 10 | 60 MB for memory efficiency; 140 MB was fastest in the six-thread main aggregate | Larger caches reduce search work slightly in opening/middlegame. Serial aggregate medians differ by less than 0.5% from 60 MB for every size. The six-thread 140 MB aggregate median time is 3.3% below 60 MB in the main study and 2.4% below in confirmation, with overlapping timing ranges and position dependence. |

The depth-10 aggregate is the geometric mean of per-position median time ratios to 60 MB, weighting each position equally. It is not summed wall time, a statistical confidence interval, or a universal optimum. At six threads, the main depth-10 relative times for 60/80/100/120/140/160 MB are 1.000/0.998/0.995/1.015/0.967/1.001. At one thread they are 1.000/0.997/0.998/1.001/0.996/1.001. The 140 MB main result prompted five further trials per position of 60 versus 140 MB because the difference was small relative to variability. See [confirmation.md](confirmation.md) and [confirmation.json](confirmation.json).

Reproduce that focused follow-up separately:

```sh
python3 benchmarks/fixed-cache/compare.py --repetitions 5 --depths 10 --threads 6 --sizes 60 140 --modes depth --output benchmarks/fixed-cache/confirmation-followup.json
```

Absolute confirmation times were higher than in the main batch. Raw times from the two batches are deliberately not pooled. Within confirmation, the opening ranking reverses, the middlegame favors 140 MB, and the endgame slightly favors 60 MB. The evidence supports a small position-dependent speed/memory tradeoff, not a definitive fastest size on all computers. More cache did not consistently raise completed depth in the two-second trials: opening stayed at 8 serial / 9 parallel, middlegame at 7 / 8, and endgame at 10 / 11–12 for every ceiling. No Elo or playing-strength claim follows from these timings.

The application uses a fixed 60 MB ceiling because it provides the best memory efficiency among the requested candidates and fits the preserved 100 MB total-server budget. A 140 MB cache alone exceeds that budget. Pages still grow on demand; this change fixes the ceiling across computers, not the amount of memory immediately committed at startup. At most eight search threads fit the existing stack/scratch/server/allocator reservations alongside that ceiling; the CPU headroom policy still applies.

The recorded base revision plus [application.patch](application.patch) reconstructs the production engine source used by the study. Preserve this benchmark directory when making a separate checkout of the recorded base, apply that patch there, and verify the resulting source hashes against the JSON before reproducing. Historical memory studies remain unchanged and continue to use their explicit total-budget constructor.

Validation: `cargo fmt -- --check`, `cargo test --offline`, `cargo clippy --all-targets --offline -- -D warnings`, `cargo build --release --offline`, `python3 tests/http_smoke.py`, and `node tests/ui_smoke.cjs` passed. The HTTP smoke test required a rerun with local-socket sandbox permission. Benchmark score and serial-repeatability assertions also passed. No builds or tests ran during timing collection.

## Smaller ceilings at depth 10

A separate follow-up tests 20 and 40 MB against fresh 60 MB controls, at depth 10 only. It uses the same three positions, one- and six-thread configurations, five cold fresh-process repetitions, and identical release-equivalent flags. The output is kept separate from the original sweep and confirmation. No equal-time trials are included because this follow-up specifically targets depth 10.

```sh
python3 benchmarks/fixed-cache/compare.py --repetitions 5 --depths 10 --sizes 20 40 60 --modes depth --output benchmarks/fixed-cache/small-depth10-followup.json
python3 benchmarks/fixed-cache/small_report.py --input benchmarks/fixed-cache/small-depth10-followup.json
```

Recorded measurements: [small-depth10.json](small-depth10.json). The [small-cache report](small-depth10.md) compares speed against the fresh 60 MB control within this batch and retains per-position timing ranges, nodes, hits, allocations, and peak RSS. Its assertions check complete depth 10, unique trial combinations, cache ceilings, score agreement, and serial repeatability. The application default is unchanged by this benchmark.

All 90 small-cache trials completed. Relative to the same-batch 60 MB control, the equal-weight geometric mean speeds across positions are:

| Cache ceiling | One thread | Six threads |
|---|---:|---:|
| 20 MB | 0.9529× (4.71% lower) | 0.9388× (6.12% lower) |
| 40 MB | 0.9935× (0.65% lower) | 1.0003× (effectively unchanged) |
| 60 MB | 1.0000× | 1.0000× |

40 MB is a promising memory/speed tradeoff in these positions; its small timing differences from 60 MB are within overlapping ranges, not evidence of a dependable speed advantage. 20 MB incurs a clearer penalty: in the serial middlegame it explores about 16.3% more nodes than 60 MB and has about 12.2% lower measured speed. 40 MB explores about 3.1% more serial middlegame nodes. The endgame never reaches even the 20 MB ceiling, so differences there reflect timing and parallel scheduling rather than a capacity benefit. Results remain position-dependent and do not predict additional depth or playing strength.

The small-cache report's score, completeness, repeatability, and allocation assertions passed. Production source hashes match the earlier study. These benchmark/documentation additions do not change engine behavior, and no application rebuild or full engine test rerun was needed.
