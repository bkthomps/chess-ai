# King pawn lanes: rejected optimization screen

This score-preserving evaluator proposal is not retained. It precomputes the three ranks ahead of each king on adjacent files and finds the nearest pawn by bit scan instead of checking each square. All shield/storm distances, weights, and other evaluation/search rules remain identical.

The immediately preceding application is revision `162cd89`. Eleven serial fresh-process pairs per position were measured at equal completed depths, cold caches, and the same 100,000,000-byte total RAM budget. Variant order was seeded and shuffled. Both binaries compiled before measurement with `rustc --edition=2021 -C opt-level=3 -C codegen-units=1 -C lto=fat`, enabling LTO as in the release profile. No task-run builds, tests, or other performance studies overlapped timing.

| Position | Median time change |
|---|---:|
| opening | +7.3% |
| after e4 | -16.5% |
| after d4 | +4.3% |
| middlegame | +1.8% |
| closed center | 0.0% |
| rook ending | -9.2% |
| tactics | +5.0% |
| pawn ending | approximately 0% |

Positive means slower. Results are mixed and timing ranges are broad; these measurements do not establish a broad improvement. No fixed-time or pondering follow-up was pursued, and no depth or playing-strength gain is claimed. See [all medians/ranges](screen.md) and [raw trials](screen.json), including source hashes, compiler/platform, FENs, actual cache allocation, and peak RSS where available.

Fixed-depth scores, best moves, serial node counts, and cache hits agree exactly between variants and repetitions. The existing suite passed for the candidate, including scalar threat/safety comparisons, symmetry, cancellation, mate, history, draw, and move-proof tests.

Reproduce the archived implementation with Git, Rust, and Python standard libraries; use a new output path:

```sh
python3 benchmarks/king-pawn-lanes/compare.py --threads 1 --modes depth --repetitions 11 --output benchmarks/king-pawn-lanes/follow-up.json
```

The driver reconstructs the frozen baseline in a temporary directory and applies `candidate.patch`. Diagnostic/benchmark entry points exist only in temporary source copies. The historical output is preserved.
