# Progress snapshot interval

Changing engine snapshot publication from 250 ms to 50 ms produced no reliably measurable slowdown in these trials. Three cold, fresh-process repetitions per interval, position, and thread configuration compared temporary engine copies differing only in the interval. The application interval was subsequently changed to 150 ms; these results measure the original 50 ms and 250 ms variants. Variant order was randomized with seed 7581. Builds used `rustc -C opt-level=3 -C codegen-units=1 -C lto`, matching the release profile's optimization and LTO settings. The default 100,000,000-byte total budget applied. The [raw data](results.json) records FENs, hashes, cache allocations, peak RSS, scores, depths, and all runs.

| Position | Threads | Fixed depth | 250 ms range (ms) | 50 ms range (ms) | Median time change |
|---|---:|---:|---:|---:|---:|
| Opening | 1 | 9 | 2333–2468 | 2341–2500 | +2.9% |
| Middlegame | 1 | 7 | 1318–1345 | 1327–1330 | −0.7% |
| Endgame | 1 | 11 | 1550–1573 | 1559–1573 | +1.0% |
| Opening | 6 | 9 | 749–825 | 761–836 | +2.9% |
| Middlegame | 6 | 7 | 469–504 | 467–557 | −5.6% |
| Endgame | 6 | 11 | 633–676 | 623–716 | +4.8% |

All fixed-depth scores agree. In two-second searches, single-thread completed depths matched for each position and interval; with six threads, opening depths matched, while middlegame and endgame varied by one ply across runs. Median nodes changed by −2.4% to +0.5% on one thread and −1.5% to +5.3% on six threads. Parallel scheduling affects node counts, and node counts include unfinished iterations. Ranges overlap and three repetitions do not resolve small effects; results are specific to this machine and positions, not an estimate of playing strength.

The temporary fixed-depth driver enables the same nonblocking progress snapshots but does not model a concurrent reader. Timed searches use the ordinary progress-enabled search API. Neither measurement includes the server, browser, or HTTP polling, so the cost of polling five times as often is not measured here. The one-thread control does not enable the single-logical-CPU yielding path. Run `python3 benchmarks/progress/compare.py` to reproduce; it overwrites `results.json`.
